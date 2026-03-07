//! Network utilities for TTstack.
//!
//! Manages the virtual network infrastructure on each host:
//! - A bridge device for VM connectivity
//! - TAP devices for individual VMs
//! - Firewall NAT rules for port forwarding
//!
//! **Linux**: uses `ip`, `nftables`
//! **FreeBSD**: uses `ifconfig`, `pf`

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use ruc::*;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use std::process::Command;

/// Default bridge name on each host.
pub const BRIDGE_NAME: &str = "tt0";
/// Bridge IP address (gateway for VMs).
pub const BRIDGE_ADDR: &str = "10.10.0.1";
/// Bridge subnet mask.
pub const BRIDGE_CIDR: &str = "10.10.0.1/16";
/// nftables table name (Linux).
#[cfg(target_os = "linux")]
pub const NFT_TABLE: &str = "tt-nat";

/// Derive an IP address for a VM from a sequential index (0..65534).
///
/// Produces addresses in the 10.10.x.y range, skipping .0 and .255.
pub fn vm_ip(index: u32) -> String {
    let index = index + 1; // skip .0.0
    let hi = (index / 254) & 0xFF;
    let lo = (index % 254) + 1;
    format!("10.10.{hi}.{lo}")
}

/// TAP device name for a VM.
///
/// Uses a hash of the VM ID to guarantee uniqueness even for long IDs.
/// Result is always <= 15 chars (IFNAMSIZ).
pub fn tap_name(vm_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    vm_id.hash(&mut h);
    let hash = h.finish();
    // "tt-" + 12 hex chars = 15 chars exactly
    format!("tt-{:012x}", hash & 0xFFFF_FFFF_FFFF)
}

// ═══════════════════════════════════════════════════════════════════
// Linux implementation
// ═══════════════════════════════════════════════════════════════════

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    pub fn setup_bridge() -> Result<()> {
        if bridge_exists()? {
            return Ok(());
        }

        run(&["ip", "link", "add", BRIDGE_NAME, "type", "bridge"])?;
        run(&["ip", "addr", "add", BRIDGE_CIDR, "dev", BRIDGE_NAME])?;
        run(&["ip", "link", "set", BRIDGE_NAME, "up"])?;

        // Enable IP forwarding
        std::fs::write("/proc/sys/net/ipv4/ip_forward", "1").c(d!("enable ip_forward"))?;

        Ok(())
    }

    pub fn bridge_exists() -> Result<bool> {
        let output = Command::new("ip")
            .args(["link", "show", BRIDGE_NAME])
            .output()
            .c(d!())?;
        Ok(output.status.success())
    }

    pub fn create_tap(vm_id: &str) -> Result<()> {
        let tap = tap_name(vm_id);

        run(&["ip", "tuntap", "add", "dev", &tap, "mode", "tap"])?;
        run(&["ip", "link", "set", &tap, "master", BRIDGE_NAME])?;
        run(&["ip", "link", "set", &tap, "up"])?;

        Ok(())
    }

    pub fn destroy_tap(vm_id: &str) -> Result<()> {
        let tap = tap_name(vm_id);
        let _ = run(&["ip", "link", "del", &tap]);
        Ok(())
    }

    pub fn setup_nat() -> Result<()> {
        nft(&format!("add table ip {NFT_TABLE}"))?;

        nft(&format!(
            "add chain ip {NFT_TABLE} prerouting {{ type nat hook prerouting priority -100; policy accept; }}"
        ))?;

        nft(&format!(
            "add chain ip {NFT_TABLE} postrouting {{ type nat hook postrouting priority 100; policy accept; }}"
        ))?;

        // Flush both chains on startup to avoid duplicate/stale rules.
        // Per-VM port forwards in prerouting will be restored from the
        // database by the agent's recovery loop.
        let _ = nft(&format!("flush chain ip {NFT_TABLE} postrouting"));
        let _ = nft(&format!("flush chain ip {NFT_TABLE} prerouting"));

        nft(&format!(
            "add rule ip {NFT_TABLE} postrouting ip saddr 10.10.0.0/16 masquerade"
        ))?;

        Ok(())
    }

    pub fn add_port_forward(host_port: u16, vm_ip_addr: &str, guest_port: u16) -> Result<()> {
        nft(&format!(
            "add rule ip {NFT_TABLE} prerouting tcp dport {host_port} dnat to {vm_ip_addr}:{guest_port}"
        ))
    }

    pub fn remove_port_forwards(vm_ip_addr: &str) -> Result<()> {
        let output = Command::new("nft")
            .args(["-a", "list", "chain", "ip", NFT_TABLE, "prerouting"])
            .output()
            .c(d!())?;

        if !output.status.success() {
            return Ok(());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        for line in listing.lines() {
            if line.contains(vm_ip_addr)
                && let Some(handle) = line
                    .rsplit("handle ")
                    .next()
                    .and_then(|h| h.trim().parse::<u64>().ok())
            {
                let _ = nft(&format!(
                    "delete rule ip {NFT_TABLE} prerouting handle {handle}"
                ));
            }
        }

        Ok(())
    }

    pub fn deny_outgoing(vm_ip_addr: &str) -> Result<()> {
        let _ = nft(&format!(
            "add set ip {NFT_TABLE} denylist {{ type ipv4_addr; }}"
        ));
        let _ = nft(&format!(
            "add chain ip {NFT_TABLE} forward {{ type filter hook forward priority 0; policy accept; }}"
        ));
        let _ = nft(&format!(
            "add rule ip {NFT_TABLE} forward ip saddr @denylist drop"
        ));

        nft(&format!(
            "add element ip {NFT_TABLE} denylist {{ {vm_ip_addr} }}"
        ))
    }

    pub fn allow_outgoing(vm_ip_addr: &str) -> Result<()> {
        let _ = nft(&format!(
            "delete element ip {NFT_TABLE} denylist {{ {vm_ip_addr} }}"
        ));
        Ok(())
    }

    fn nft(rule: &str) -> Result<()> {
        use std::io::Write;
        let mut child = Command::new("nft")
            .arg("-f")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .c(d!("nft spawn"))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(rule.as_bytes());
            let _ = stdin.write_all(b"\n");
        }

        let output = child.wait_with_output().c(d!("nft wait"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("nft {}: {}", rule, stderr));
        }

        Ok(())
    }

    fn run(args: &[&str]) -> Result<()> {
        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .c(d!(args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("{}: {}", args.join(" "), stderr));
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// FreeBSD implementation
// ═══════════════════════════════════════════════════════════════════

#[cfg(target_os = "freebsd")]
mod platform {
    use super::*;

    pub fn setup_bridge() -> Result<()> {
        if bridge_exists()? {
            return Ok(());
        }

        run(&["ifconfig", "bridge", "create", "name", BRIDGE_NAME])?;
        run(&["ifconfig", BRIDGE_NAME, "inet", BRIDGE_CIDR])?;
        run(&["ifconfig", BRIDGE_NAME, "up"])?;

        // Enable IP forwarding
        run(&["sysctl", "net.inet.ip.forwarding=1"])?;

        Ok(())
    }

    pub fn bridge_exists() -> Result<bool> {
        let output = Command::new("ifconfig").arg(BRIDGE_NAME).output().c(d!())?;
        Ok(output.status.success())
    }

    pub fn create_tap(vm_id: &str) -> Result<()> {
        let tap = tap_name(vm_id);

        run(&["ifconfig", "tap", "create", "name", &tap])?;
        run(&["ifconfig", BRIDGE_NAME, "addm", &tap])?;
        run(&["ifconfig", &tap, "up"])?;

        Ok(())
    }

    pub fn destroy_tap(vm_id: &str) -> Result<()> {
        let tap = tap_name(vm_id);
        let _ = run(&["ifconfig", &tap, "destroy"]);
        Ok(())
    }

    pub fn setup_nat() -> Result<()> {
        // PF should be configured in /etc/pf.conf
        // We only enable it here
        let _ = run(&["pfctl", "-e"]);
        Ok(())
    }

    pub fn add_port_forward(host_port: u16, vm_ip_addr: &str, guest_port: u16) -> Result<()> {
        // Add a PF rdr rule via pfctl
        let rule = format!(
            "rdr pass on egress proto tcp from any to any port {host_port} -> {vm_ip_addr} port {guest_port}"
        );
        let output = Command::new("sh")
            .args(["-c", &format!(r#"echo '{rule}' | pfctl -a ttstack -f -"#)])
            .output()
            .c(d!("pfctl rdr"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("pfctl rdr failed: {}", stderr));
        }
        Ok(())
    }

    pub fn remove_port_forwards(vm_ip_addr: &str) -> Result<()> {
        // List current rules and remove only those matching this VM's IP
        let output = Command::new("pfctl")
            .args(["-a", "ttstack", "-s", "rules"])
            .output();

        if let Ok(output) = output {
            let rules = String::from_utf8_lossy(&output.stdout);
            let remaining: Vec<&str> = rules
                .lines()
                .filter(|line| !line.contains(vm_ip_addr))
                .collect();

            if remaining.is_empty() {
                // No rules left — flush the anchor
                let _ = run(&["pfctl", "-a", "ttstack", "-F", "rules"]);
            } else {
                // Reload only the remaining rules
                let new_rules = remaining.join("\n");
                let _ = Command::new("sh")
                    .args([
                        "-c",
                        &format!(r#"echo '{}' | pfctl -a ttstack -f -"#, new_rules),
                    ])
                    .output();
            }
        }

        Ok(())
    }

    pub fn deny_outgoing(vm_ip_addr: &str) -> Result<()> {
        let rule = format!("block out quick on egress from {vm_ip_addr} to any");
        let output = Command::new("sh")
            .args([
                "-c",
                &format!(r#"echo '{rule}' | pfctl -a ttstack/deny -f -"#),
            ])
            .output()
            .c(d!("pfctl deny"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("pfctl deny failed: {}", stderr));
        }
        Ok(())
    }

    pub fn allow_outgoing(vm_ip_addr: &str) -> Result<()> {
        // List current deny rules and remove only those matching this VM's IP
        let output = Command::new("pfctl")
            .args(["-a", "ttstack/deny", "-s", "rules"])
            .output();

        if let Ok(output) = output {
            let rules = String::from_utf8_lossy(&output.stdout);
            let remaining: Vec<&str> = rules
                .lines()
                .filter(|line| !line.contains(vm_ip_addr))
                .collect();

            if remaining.is_empty() {
                let _ = run(&["pfctl", "-a", "ttstack/deny", "-F", "rules"]);
            } else {
                let new_rules = remaining.join("\n");
                let _ = Command::new("sh")
                    .args([
                        "-c",
                        &format!(r#"echo '{}' | pfctl -a ttstack/deny -f -"#, new_rules),
                    ])
                    .output();
            }
        }

        Ok(())
    }

    fn run(args: &[&str]) -> Result<()> {
        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .c(d!(args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("{}: {}", args.join(" "), stderr));
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// Public re-exports (dispatches to platform module)
// ═══════════════════════════════════════════════════════════════════

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn setup_bridge() -> Result<()> {
    platform::setup_bridge()
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn setup_nat() -> Result<()> {
    platform::setup_nat()
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn create_tap(vm_id: &str, _vm_ip_addr: &str) -> Result<()> {
    platform::create_tap(vm_id)
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn destroy_tap(vm_id: &str) -> Result<()> {
    platform::destroy_tap(vm_id)
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn add_port_forward(host_port: u16, vm_ip_addr: &str, guest_port: u16) -> Result<()> {
    platform::add_port_forward(host_port, vm_ip_addr, guest_port)
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn remove_port_forwards(vm_ip_addr: &str) -> Result<()> {
    platform::remove_port_forwards(vm_ip_addr)
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn deny_outgoing(vm_ip_addr: &str) -> Result<()> {
    platform::deny_outgoing(vm_ip_addr)
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn allow_outgoing(vm_ip_addr: &str) -> Result<()> {
    platform::allow_outgoing(vm_ip_addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_ip_first() {
        // index=0 → internal=1 → hi=0, lo=2 → 10.10.0.2
        assert_eq!(vm_ip(0), "10.10.0.2");
    }

    #[test]
    fn vm_ip_sequential() {
        assert_eq!(vm_ip(1), "10.10.0.3");
        // index=252 → internal=253 → hi=0, lo=254 → 10.10.0.254
        assert_eq!(vm_ip(252), "10.10.0.254");
    }

    #[test]
    fn vm_ip_wraps_to_next_octet() {
        // index=253 → internal=254 → hi=1, lo=254%254+1=1 → 10.10.1.1
        assert_eq!(vm_ip(253), "10.10.1.1");
    }

    #[test]
    fn vm_ip_unique_and_valid() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for i in 0..1000 {
            let ip = vm_ip(i);
            assert!(seen.insert(ip.clone()), "duplicate IP at index {i}: {ip}");
            // Verify no .0 or .255 in last octet
            let lo: u32 = ip.rsplit('.').next().unwrap().parse().unwrap();
            assert!(lo >= 1 && lo <= 254, "invalid lo octet {lo} at index {i}");
        }
    }

    #[test]
    fn tap_name_fits_ifnamsiz() {
        assert!(tap_name("abc").len() <= 15);
        assert!(tap_name("a".repeat(200).as_str()).len() <= 15);
    }

    #[test]
    fn tap_name_deterministic() {
        assert_eq!(tap_name("vm1"), tap_name("vm1"));
    }

    #[test]
    fn tap_name_unique_for_different_ids() {
        assert_ne!(tap_name("vm1"), tap_name("vm2"));
        // Long IDs that used to collide via truncation are now unique
        assert_ne!(
            tap_name("very_long_vm_name_1"),
            tap_name("very_long_vm_name_2")
        );
    }
}
