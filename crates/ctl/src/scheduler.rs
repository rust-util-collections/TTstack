//! VM placement scheduler.
//!
//! Decides which host should run each VM based on available resources,
//! supported engines, and a simple best-fit strategy.

use ruc::*;
use ttcore::api::VmSpec;
use ttcore::model::*;

/// Result of scheduling: VM spec + chosen host.
pub struct Placement {
    pub host_id: String,
    pub host_addr: String,
}

/// Choose the best host for a VM spec using a best-fit strategy.
///
/// Prefers the host with the least free resources that can still
/// accommodate the VM, to pack hosts densely and leave larger hosts
/// available for bigger workloads.
pub fn place_vm(hosts: &[Host], spec: &VmSpec) -> Result<Placement> {
    let cpu = spec.cpu.unwrap_or(VM_CPU_DEFAULT);
    let mem = spec.mem.unwrap_or(VM_MEM_DEFAULT);
    let disk = spec.disk.unwrap_or(VM_DISK_DEFAULT);

    let mut candidates: Vec<&Host> = hosts
        .iter()
        .filter(|h| {
            h.state == HostState::Online
                && h.engines.contains(&spec.engine)
                && h.resource.can_fit(cpu, mem, disk)
        })
        .collect();

    if candidates.is_empty() {
        return Err(eg!(
            "no host available for engine={}, cpu={}, mem={}MB, disk={}MB",
            spec.engine,
            cpu,
            mem,
            disk,
        ));
    }

    // Sort by free memory ascending (best-fit)
    candidates.sort_by_key(|h| h.resource.mem_free());

    let host = candidates[0];
    Ok(Placement {
        host_id: host.id.clone(),
        host_addr: host.addr.clone(),
    })
}

/// Schedule an entire environment's VMs across the fleet.
///
/// Returns a list of (VmSpec, Placement) pairs.
pub fn schedule_env(
    hosts: &[Host],
    specs: &[VmSpec],
) -> Result<Vec<(VmSpec, Placement)>> {
    let mut result = Vec::with_capacity(specs.len());

    // Work with a mutable copy of host resources for multi-VM scheduling
    let mut shadow: Vec<Host> = hosts.to_vec();

    for spec in specs {
        let placement = place_vm(&shadow, spec)?;

        // Update shadow resources to account for this allocation
        if let Some(h) = shadow.iter_mut().find(|h| h.id == placement.host_id) {
            let cpu = spec.cpu.unwrap_or(VM_CPU_DEFAULT);
            let mem = spec.mem.unwrap_or(VM_MEM_DEFAULT);
            let disk = spec.disk.unwrap_or(VM_DISK_DEFAULT);
            h.resource.cpu_used += cpu;
            h.resource.mem_used += mem;
            h.resource.disk_used += disk;
            h.resource.vm_count += 1;
        }

        result.push((spec.clone(), placement));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_host(id: &str, cpu: u32, mem: u32, engines: Vec<Engine>) -> Host {
        Host {
            id: id.into(),
            addr: format!("{id}:9100"),
            resource: Resource {
                cpu_total: cpu,
                cpu_used: 0,
                mem_total: mem,
                mem_used: 0,
                disk_total: 500_000,
                disk_used: 0,
                vm_count: 0,
            },
            state: HostState::Online,
            engines,
            storage: Storage::Raw,
            registered_at: 0,
        }
    }

    fn make_spec() -> VmSpec {
        VmSpec {
            image: "ubuntu".into(),
            engine: Engine::Qemu,
            cpu: Some(2),
            mem: Some(1024),
            disk: Some(40960),
            ports: vec![22],
            deny_outgoing: false,
        }
    }

    #[test]
    fn place_vm_picks_online_host() {
        let hosts = vec![make_host("h1", 8, 16384, vec![Engine::Qemu])];
        let p = place_vm(&hosts, &make_spec()).unwrap();
        assert_eq!(p.host_id, "h1");
    }

    #[test]
    fn place_vm_skips_offline_host() {
        let mut h = make_host("h1", 8, 16384, vec![Engine::Qemu]);
        h.state = HostState::Offline;
        let hosts = vec![h];
        assert!(place_vm(&hosts, &make_spec()).is_err());
    }

    #[test]
    fn place_vm_skips_wrong_engine() {
        let hosts = vec![make_host("h1", 8, 16384, vec![Engine::Docker])];
        let spec = make_spec(); // wants Qemu
        assert!(place_vm(&hosts, &spec).is_err());
    }

    #[test]
    fn place_vm_skips_insufficient_resources() {
        let hosts = vec![make_host("h1", 1, 512, vec![Engine::Qemu])];
        let spec = make_spec(); // needs 2 CPU, 1024 mem
        assert!(place_vm(&hosts, &spec).is_err());
    }

    #[test]
    fn place_vm_best_fit_prefers_smaller() {
        // h1 has more room, h2 has less but enough
        let hosts = vec![
            make_host("h1", 16, 32768, vec![Engine::Qemu]),
            make_host("h2", 4, 4096, vec![Engine::Qemu]),
        ];
        let p = place_vm(&hosts, &make_spec()).unwrap();
        assert_eq!(p.host_id, "h2"); // best-fit picks smaller
    }

    #[test]
    fn schedule_env_distributes_when_full() {
        let hosts = vec![
            make_host("h1", 4, 4096, vec![Engine::Qemu]),
            make_host("h2", 4, 4096, vec![Engine::Qemu]),
        ];

        // 3 VMs each needing 2 CPU: h1 takes 2 (filling up), h2 takes 1
        let specs: Vec<VmSpec> = (0..3).map(|_| make_spec()).collect();
        let placements = schedule_env(&hosts, &specs).unwrap();
        assert_eq!(placements.len(), 3);

        let on_h1 = placements.iter().filter(|(_, p)| p.host_id == "h1").count();
        let on_h2 = placements.iter().filter(|(_, p)| p.host_id == "h2").count();
        assert_eq!(on_h1, 2);
        assert_eq!(on_h2, 1);
    }

    #[test]
    fn schedule_env_fails_if_no_capacity() {
        let hosts = vec![make_host("h1", 2, 2048, vec![Engine::Qemu])];
        let specs: Vec<VmSpec> = (0..2).map(|_| make_spec()).collect();
        assert!(schedule_env(&hosts, &specs).is_err());
    }

    #[test]
    fn schedule_env_empty_specs_ok() {
        let hosts = vec![make_host("h1", 8, 16384, vec![Engine::Qemu])];
        let placements = schedule_env(&hosts, &[]).unwrap();
        assert!(placements.is_empty());
    }
}
