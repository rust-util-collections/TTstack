//! VM runtime management on a single host.
//!
//! Owns the lifecycle of all VMs on this host, backed by SQLite for
//! crash-recoverable persistent state.

use ruc::*;
use rusqlite::Connection;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use ttcore::api::{AgentInfo, CreateVmReq};
use ttcore::engine;
use ttcore::model::*;
use ttcore::net;
use ttcore::storage::{self, ImageStore};

/// Manages all VMs on this host.
pub struct Runtime {
    pub host_id: String,
    db: Connection,
    engines: Vec<Engine>,
    store: Box<dyn ImageStore>,
    storage: Storage,
    image_dir: String,
    runtime_dir: String,
    pub resource: Resource,
    next_ip_idx: AtomicU32,
}

impl Runtime {
    /// Initialize the runtime, restoring state from SQLite if available.
    pub fn new(
        host_id: String,
        storage: Storage,
        image_dir: String,
        runtime_dir: String,
        db_path: &str,
        resource: Resource,
    ) -> Result<Self> {
        std::fs::create_dir_all(&image_dir).c(d!("create image_dir"))?;
        std::fs::create_dir_all(&runtime_dir).c(d!("create runtime_dir"))?;
        std::fs::create_dir_all(ttcore::model::RUN_DIR).c(d!("create run dir"))?;

        let db = Connection::open(db_path).c(d!("open agent db"))?;
        init_db(&db)?;

        let store = storage::create_store(storage);
        let engines = detect_engines();

        // Restore state from database
        let vms = load_all_vms(&db)?;

        // Clean up VMs stuck in "Creating" state (agent crashed mid-creation).
        // These VMs may have partial resources allocated that need cleanup.
        for vm in &vms {
            if vm.state == VmState::Creating {
                eprintln!(
                    "[agent] cleaning up orphaned VM {} (stuck in Creating state)",
                    vm.id
                );
                let eng = engine::create_engine(vm.engine);
                let _ = eng.destroy(vm);
                let _ = storage::create_store(storage)
                    .remove_image(&format!("{}/clone-{}", runtime_dir, vm.id));
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                net::destroy_tap(&vm.id).unwrap_or(());
                let _ = delete_vm(&db, &vm.id);
            }
        }

        // Reload after cleanup
        let vms = load_all_vms(&db)?;

        let max_idx = vms
            .iter()
            .filter_map(|vm| ip_to_index(&vm.ip))
            .max()
            .unwrap_or(0);

        let mut cpu_used = 0u32;
        let mut mem_used = 0u32;
        let mut disk_used = 0u32;
        let mut vm_count = 0u32;
        for vm in &vms {
            if vm.state == VmState::Running || vm.state == VmState::Paused {
                cpu_used += vm.cpu;
                mem_used += vm.mem;
                disk_used += vm.disk;
                vm_count += 1;
            }
        }

        let resource = Resource {
            cpu_used,
            mem_used,
            disk_used,
            vm_count,
            ..resource
        };

        // Set up networking (only on platforms with host-managed networking)
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            net::setup_bridge().c(d!("bridge setup"))?;
            net::setup_nat().c(d!("NAT setup"))?;
        }

        // Restore network rules for persisted VMs
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        for vm in &vms {
            if vm.state == VmState::Running || vm.state == VmState::Paused {
                if let Err(e) = net::create_tap(&vm.id, &vm.ip) {
                    eprintln!("[agent] WARN: failed to restore TAP for VM {}: {e}", vm.id);
                }
                for (&guest, &host) in &vm.port_map {
                    if let Err(e) = net::add_port_forward(host, &vm.ip, guest) {
                        eprintln!(
                            "[agent] WARN: failed to restore port forward {}->{}:{} for VM {}: {e}",
                            host, vm.ip, guest, vm.id
                        );
                    }
                }
            }
        }

        Ok(Self {
            host_id,
            db,
            engines,
            store,
            storage,
            image_dir,
            runtime_dir,
            resource,
            next_ip_idx: AtomicU32::new(max_idx + 1),
        })
    }

    /// Create a new VM.
    pub fn create_vm(&mut self, req: &CreateVmReq) -> Result<Vm> {
        // Input validation
        validate_name(&req.vm_id, "vm_id").map_err(|e| eg!(e))?;
        validate_name(&req.image, "image").map_err(|e| eg!(e))?;

        if req.cpu == 0 {
            return Err(eg!("cpu must be > 0"));
        }
        if req.mem == 0 {
            return Err(eg!("mem must be > 0"));
        }
        if req.disk == 0 {
            return Err(eg!("disk must be > 0"));
        }

        if !self.resource.can_fit(req.cpu, req.mem, req.disk) {
            return Err(eg!("insufficient resources on host {}", self.host_id));
        }

        if self.resource.vm_count as usize >= MAX_VMS {
            return Err(eg!("VM limit reached"));
        }

        // Allocate IP, skipping any indices already in use by existing VMs
        let existing_ips: HashSet<String> = load_all_vms(&self.db)
            .unwrap_or_default()
            .into_iter()
            .map(|vm| vm.ip)
            .collect();
        let (ip_idx, ip) = loop {
            let idx = self.next_ip_idx.fetch_add(1, Ordering::SeqCst);
            if idx > 65000 {
                return Err(eg!("IP address space exhausted"));
            }
            let candidate = net::vm_ip(idx);
            if !existing_ips.contains(&candidate) {
                break (idx, candidate);
            }
        };

        // Docker/Podman manages its own images, networking, and port mapping;
        // other engines need local image clones, TAP devices, and nftables rules.
        // Host-managed networking is only available on Linux and FreeBSD.
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        let host_managed_net = req.engine != Engine::Docker;
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        let host_managed_net = false;

        let clone_path = format!("{}/clone-{}", self.runtime_dir, req.vm_id);
        if host_managed_net {
            let base_image = format!("{}/{}", self.image_dir, req.image);
            self.store
                .clone_image(&base_image, &clone_path)
                .c(d!("image clone"))?;
        }

        if host_managed_net {
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            net::create_tap(&req.vm_id, &ip).c(d!("TAP setup"))?;
        }

        // Allocate port mappings (use checked arithmetic to avoid u16 overflow)
        // Always include port 22 for SSH access
        let mut ports = req.ports.clone();
        if !ports.contains(&22) {
            ports.insert(0, 22);
        }
        let mut port_map = BTreeMap::new();
        let base_port = 20000u32.saturating_add(ip_idx.saturating_mul(100));
        for (i, &guest_port) in ports.iter().enumerate() {
            let host_port = base_port.saturating_add(i as u32);
            if host_port > 65535 {
                break; // silently skip ports that can't be allocated
            }
            port_map.insert(guest_port, host_port as u16);
        }

        let mut vm = Vm {
            id: req.vm_id.clone(),
            env_id: req.env_id.clone(),
            host_id: self.host_id.clone(),
            image: req.image.clone(),
            engine: req.engine,
            cpu: req.cpu,
            mem: req.mem,
            disk: req.disk,
            ip: ip.clone(),
            port_map: port_map.clone(),
            state: VmState::Creating,
            created_at: now(),
        };

        save_vm(&self.db, &vm)?;

        // Resolve the disk path and format from the storage backend
        let disk_path = self.store.resolve_disk(&clone_path);
        let disk_format = self.store.disk_format();

        // Launch using the appropriate engine
        let eng = engine::create_engine(req.engine);
        if let Err(e) = eng.create(&vm, &disk_path, disk_format, &req.ssh_keys) {
            if host_managed_net {
                let _ = self.store.remove_image(&clone_path);
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                net::destroy_tap(&req.vm_id).unwrap_or(());
            }
            delete_vm(&self.db, &vm.id)?;
            return Err(e).c(d!("engine create"));
        }

        // Set up nftables port forwarding and outgoing rules.
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if host_managed_net {
            let post_create = || -> Result<()> {
                for (&guest_port, &host_port) in &port_map {
                    net::add_port_forward(host_port, &ip, guest_port).c(d!("port forward"))?;
                }
                if req.deny_outgoing {
                    net::deny_outgoing(&ip).c(d!("deny outgoing"))?;
                }
                Ok(())
            };

            if let Err(e) = post_create() {
                let _ = net::remove_port_forwards(&ip);
                let _ = net::allow_outgoing(&ip);
                let _ = eng.destroy(&vm);
                let _ = self.store.remove_image(&clone_path);
                let _ = net::destroy_tap(&req.vm_id);
                let _ = delete_vm(&self.db, &vm.id);
                return Err(e).c(d!("post-create setup"));
            }
        }

        // Update resource tracking
        self.resource.cpu_used += req.cpu;
        self.resource.mem_used += req.mem;
        self.resource.disk_used += req.disk;
        self.resource.vm_count += 1;

        vm.state = VmState::Running;
        save_vm(&self.db, &vm)?;

        Ok(vm)
    }

    pub fn stop_vm(&mut self, vm_id: &str) -> Result<()> {
        let mut vm = load_vm(&self.db, vm_id)?.ok_or_else(|| eg!("VM not found: {}", vm_id))?;

        match vm.state {
            VmState::Running => {}
            VmState::Paused | VmState::Stopped => return Ok(()),
            _ => return Err(eg!("cannot stop VM in state {}", vm.state)),
        }

        let eng = engine::create_engine(vm.engine);
        eng.stop(&vm).c(d!("stop VM"))?;

        // FC/QEMU pause (preserve process); others fully stop
        let pauses = matches!(vm.engine, Engine::Qemu | Engine::Firecracker);
        if pauses {
            vm.state = VmState::Paused;
        } else {
            vm.state = VmState::Stopped;
            self.resource.cpu_used = self.resource.cpu_used.saturating_sub(vm.cpu);
            self.resource.mem_used = self.resource.mem_used.saturating_sub(vm.mem);
        }
        save_vm(&self.db, &vm)?;

        Ok(())
    }

    pub fn start_vm(&mut self, vm_id: &str) -> Result<()> {
        let mut vm = load_vm(&self.db, vm_id)?.ok_or_else(|| eg!("VM not found: {}", vm_id))?;

        let prev_state = vm.state;
        match prev_state {
            VmState::Stopped | VmState::Paused => {}
            VmState::Running => return Ok(()),
            _ => return Err(eg!("cannot start VM in state {}", vm.state)),
        }

        // Only check/allocate resources when resuming from fully Stopped
        if prev_state == VmState::Stopped && !self.resource.can_fit(vm.cpu, vm.mem, 0) {
            return Err(eg!("insufficient resources to restart VM"));
        }

        let eng = engine::create_engine(vm.engine);
        eng.start(&vm).c(d!("start VM"))?;

        vm.state = VmState::Running;
        save_vm(&self.db, &vm)?;

        if prev_state == VmState::Stopped {
            self.resource.cpu_used += vm.cpu;
            self.resource.mem_used += vm.mem;
        }

        Ok(())
    }

    pub fn destroy_vm(&mut self, vm_id: &str) -> Result<()> {
        let vm = match load_vm(&self.db, vm_id)? {
            Some(vm) => vm,
            None => return Ok(()),
        };

        let eng = engine::create_engine(vm.engine);
        let _ = eng.destroy(&vm);

        // Clean up host-managed networking and image clones.
        // Docker handles its own network teardown.
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if vm.engine != Engine::Docker {
            net::remove_port_forwards(&vm.ip).unwrap_or(());
            net::allow_outgoing(&vm.ip).unwrap_or(());
            net::destroy_tap(vm_id).unwrap_or(());
        }

        if vm.engine != Engine::Docker {
            let clone_path = format!("{}/clone-{}", self.runtime_dir, vm_id);
            let _ = self.store.remove_image(&clone_path);
        }

        if vm.state == VmState::Running
            || vm.state == VmState::Paused
            || vm.state == VmState::Creating
        {
            self.resource.cpu_used = self.resource.cpu_used.saturating_sub(vm.cpu);
            self.resource.mem_used = self.resource.mem_used.saturating_sub(vm.mem);
        }
        self.resource.disk_used = self.resource.disk_used.saturating_sub(vm.disk);
        self.resource.vm_count = self.resource.vm_count.saturating_sub(1);

        delete_vm(&self.db, vm_id)?;

        Ok(())
    }

    pub fn get_vm(&self, vm_id: &str) -> Option<Vm> {
        load_vm(&self.db, vm_id).ok().flatten()
    }

    pub fn list_vms(&self) -> Vec<Vm> {
        load_all_vms(&self.db).unwrap_or_default()
    }

    pub fn list_images(&self) -> Vec<String> {
        self.store.list_images(&self.image_dir).unwrap_or_default()
    }

    pub fn agent_info(&self) -> AgentInfo {
        AgentInfo {
            host_id: self.host_id.clone(),
            resource: self.resource.clone(),
            engines: self.engines.clone(),
            storage: self.storage,
            images: self.list_images(),
        }
    }
}

// ── SQLite Schema & Operations ──────────────────────────────────────

/// Current agent schema version.
const SCHEMA_VERSION: u32 = 1;

fn init_db(db: &Connection) -> Result<()> {
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS _meta (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );",
    )
    .c(d!("init meta table"))?;

    let current = get_schema_version(db)?;

    if current > SCHEMA_VERSION {
        return Err(eg!(
            "agent DB schema v{} is newer than this binary (v{}); upgrade TTstack first",
            current,
            SCHEMA_VERSION
        ));
    }

    if current < 1 {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS vms (
                id       TEXT PRIMARY KEY,
                data     TEXT NOT NULL
            );",
        )
        .c(d!("migration v1"))?;
    }

    // Future migrations: if current < 2 { ... }

    set_schema_version(db, SCHEMA_VERSION)?;
    if current < SCHEMA_VERSION {
        eprintln!("agent DB migrated: v{current} → v{SCHEMA_VERSION}");
    }

    Ok(())
}

fn get_schema_version(db: &Connection) -> Result<u32> {
    let mut stmt = db
        .prepare("SELECT value FROM _meta WHERE key = 'schema_version'")
        .c(d!())?;
    let mut rows = stmt.query([]).c(d!())?;
    match rows.next().c(d!())? {
        Some(row) => {
            let val: String = row.get(0).c(d!())?;
            val.parse::<u32>()
                .map_err(|_| eg!("invalid schema_version: {val}"))
        }
        None => Ok(0),
    }
}

fn set_schema_version(db: &Connection, ver: u32) -> Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO _meta (key, value) VALUES ('schema_version', ?1)",
        rusqlite::params![ver.to_string()],
    )
    .c(d!("set schema version"))?;
    Ok(())
}

/// Load or generate a stable host_id persisted in the agent database.
///
/// If `--host-id` is provided on the CLI, that value takes precedence and
/// is saved for future restarts. Otherwise we check the database; only
/// when neither is available do we generate a new random ID.
pub fn resolve_host_id(db_path: &str, cli_id: Option<String>) -> Result<String> {
    let conn = Connection::open(db_path).c(d!("open DB for host_id"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _meta (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );",
    )
    .c(d!("ensure meta table"))?;

    if let Some(id) = cli_id {
        // CLI takes precedence — persist it
        conn.execute(
            "INSERT OR REPLACE INTO _meta (key, value) VALUES ('host_id', ?1)",
            rusqlite::params![id],
        )
        .c(d!("persist CLI host_id"))?;
        return Ok(id);
    }

    // Try to load from DB
    let mut stmt = conn
        .prepare("SELECT value FROM _meta WHERE key = 'host_id'")
        .c(d!())?;
    let mut rows = stmt.query([]).c(d!())?;
    if let Some(row) = rows.next().c(d!())? {
        let val: String = row.get(0).c(d!())?;
        return Ok(val);
    }
    drop(rows);
    drop(stmt);

    // Generate and persist a new ID
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    conn.execute(
        "INSERT OR REPLACE INTO _meta (key, value) VALUES ('host_id', ?1)",
        rusqlite::params![id],
    )
    .c(d!("persist generated host_id"))?;
    Ok(id)
}

fn save_vm(db: &Connection, vm: &Vm) -> Result<()> {
    let data = serde_json::to_string(vm).c(d!("serialize VM"))?;
    db.execute(
        "INSERT OR REPLACE INTO vms (id, data) VALUES (?1, ?2)",
        rusqlite::params![vm.id, data],
    )
    .c(d!("save VM"))?;
    Ok(())
}

fn load_vm(db: &Connection, id: &str) -> Result<Option<Vm>> {
    let mut stmt = db
        .prepare("SELECT data FROM vms WHERE id = ?1")
        .c(d!("prepare load VM"))?;
    let mut rows = stmt.query(rusqlite::params![id]).c(d!("query VM"))?;
    match rows.next().c(d!("next row"))? {
        Some(row) => {
            let data: String = row.get(0).c(d!("get data"))?;
            let vm: Vm = serde_json::from_str(&data).c(d!("deserialize VM"))?;
            Ok(Some(vm))
        }
        None => Ok(None),
    }
}

fn load_all_vms(db: &Connection) -> Result<Vec<Vm>> {
    let mut stmt = db
        .prepare("SELECT data FROM vms")
        .c(d!("prepare list VMs"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .c(d!("query all VMs"))?;
    let mut vms = Vec::new();
    for row in rows {
        let data = row.c(d!("read row"))?;
        let vm: Vm = serde_json::from_str(&data).c(d!("deserialize VM"))?;
        vms.push(vm);
    }
    Ok(vms)
}

fn delete_vm(db: &Connection, id: &str) -> Result<()> {
    db.execute("DELETE FROM vms WHERE id = ?1", rusqlite::params![id])
        .c(d!("delete VM"))?;
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn detect_engines() -> Vec<Engine> {
    let mut engines = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if which("qemu-system-x86_64") {
            engines.push(Engine::Qemu);
        }
        if which("firecracker") {
            engines.push(Engine::Firecracker);
        }
    }

    #[cfg(target_os = "freebsd")]
    {
        if which("bhyve") {
            engines.push(Engine::Bhyve);
        }
        if which("jail") {
            engines.push(Engine::Jail);
        }
    }

    // Docker/Podman is available on all platforms
    if which("docker") || which("podman") {
        engines.push(Engine::Docker);
    }

    engines
}

fn which(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ip_to_index(ip: &str) -> Option<u32> {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let hi: u32 = parts[2].parse().ok()?;
    let lo: u32 = parts[3].parse().ok()?;
    // Inverse of vm_ip: internal = hi*254 + (lo-1), index = internal - 1
    let internal = hi * 254 + lo.checked_sub(1)?;
    internal.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttcore::net;

    #[test]
    fn ip_to_index_first() {
        let ip = net::vm_ip(0);
        assert_eq!(ip_to_index(&ip), Some(0));
    }

    #[test]
    fn ip_to_index_sequential() {
        for i in 0..100 {
            let ip = net::vm_ip(i);
            assert_eq!(ip_to_index(&ip), Some(i), "mismatch at index {i}: ip={ip}");
        }
    }

    #[test]
    fn ip_to_index_cross_octet() {
        let ip = net::vm_ip(253);
        let roundtrip = ip_to_index(&ip);
        assert_eq!(roundtrip, Some(253), "ip={ip} roundtrip={roundtrip:?}");
    }

    #[test]
    fn ip_to_index_invalid() {
        assert_eq!(ip_to_index("not-an-ip"), None);
        assert_eq!(ip_to_index("10.10.0"), None);
        assert_eq!(ip_to_index("10.10.x.1"), None);
    }

    // ── SQLite DB operations ────────────────────────────────────────

    fn test_db() -> Connection {
        let db = Connection::open(":memory:").unwrap();
        init_db(&db).unwrap();
        db
    }

    fn make_vm(id: &str, state: VmState) -> Vm {
        Vm {
            id: id.into(),
            env_id: "env1".into(),
            host_id: "h1".into(),
            image: "ubuntu".into(),
            engine: Engine::Qemu,
            cpu: 2,
            mem: 1024,
            disk: 40960,
            ip: "10.10.0.2".into(),
            port_map: BTreeMap::new(),
            state,
            created_at: 1000,
        }
    }

    #[test]
    fn db_save_and_load_vm() {
        let db = test_db();
        let vm = make_vm("vm1", VmState::Running);
        save_vm(&db, &vm).unwrap();

        let loaded = load_vm(&db, "vm1").unwrap().unwrap();
        assert_eq!(loaded.id, "vm1");
        assert_eq!(loaded.state, VmState::Running);
        assert_eq!(loaded.cpu, 2);
    }

    #[test]
    fn db_load_nonexistent_vm() {
        let db = test_db();
        let result = load_vm(&db, "nope").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn db_load_all_vms() {
        let db = test_db();
        save_vm(&db, &make_vm("a", VmState::Running)).unwrap();
        save_vm(&db, &make_vm("b", VmState::Stopped)).unwrap();
        let all = load_all_vms(&db).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn db_delete_vm() {
        let db = test_db();
        save_vm(&db, &make_vm("vm1", VmState::Running)).unwrap();
        delete_vm(&db, "vm1").unwrap();
        assert!(load_vm(&db, "vm1").unwrap().is_none());
    }

    #[test]
    fn db_delete_nonexistent_vm() {
        let db = test_db();
        // Should not error
        delete_vm(&db, "nope").unwrap();
    }

    #[test]
    fn db_upsert_vm() {
        let db = test_db();
        let mut vm = make_vm("vm1", VmState::Creating);
        save_vm(&db, &vm).unwrap();

        vm.state = VmState::Running;
        save_vm(&db, &vm).unwrap();

        let loaded = load_vm(&db, "vm1").unwrap().unwrap();
        assert_eq!(loaded.state, VmState::Running);
        // Only one row
        assert_eq!(load_all_vms(&db).unwrap().len(), 1);
    }

    #[test]
    fn db_schema_version_persisted() {
        let db = test_db();
        let ver = get_schema_version(&db).unwrap();
        assert_eq!(ver, SCHEMA_VERSION);
    }

    // ── resolve_host_id ────────────────────────────────────────────

    #[test]
    fn resolve_host_id_generates_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.to_str().unwrap();

        let id1 = resolve_host_id(path_str, None).unwrap();
        assert!(!id1.is_empty());
        assert!(id1.len() <= 8);

        // Second call returns the same persisted ID
        let id2 = resolve_host_id(path_str, None).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn resolve_host_id_cli_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.to_str().unwrap();

        let id1 = resolve_host_id(path_str, None).unwrap();
        let id2 = resolve_host_id(path_str, Some("custom-id".into())).unwrap();
        assert_eq!(id2, "custom-id");
        assert_ne!(id1, id2);

        // Persisted the CLI override
        let id3 = resolve_host_id(path_str, None).unwrap();
        assert_eq!(id3, "custom-id");
    }
}
