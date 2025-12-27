//! Agent configuration.

use clap::Parser;
use ttcore::model::Storage;

/// TTstack host agent — manages VMs and containers on this host.
#[derive(Parser, Debug)]
#[command(name = "tt-agent", version)]
pub struct Config {
    /// Listen address for the HTTP API.
    #[arg(long, default_value = "0.0.0.0:9100")]
    pub listen: String,

    /// Directory containing base VM/container images.
    #[arg(long, default_value = "/home/ttstack/images")]
    pub image_dir: String,

    /// Directory for runtime VM image clones.
    #[arg(long, default_value = "/home/ttstack/runtime")]
    pub runtime_dir: String,

    /// Directory for persistent state (SQLite database).
    #[arg(long, default_value = "/home/ttstack/data")]
    pub data_dir: String,

    /// Storage backend: zfs, btrfs, or raw.
    #[arg(long, default_value = "raw")]
    pub storage: String,

    /// Total CPU cores available for VMs (0 = auto-detect).
    #[arg(long, default_value_t = 0)]
    pub cpu_total: u32,

    /// Total memory for VMs in MiB (0 = auto-detect).
    #[arg(long, default_value_t = 0)]
    pub mem_total: u32,

    /// Total disk for VMs in MiB (default: ~200 GiB).
    #[arg(long, default_value_t = 200 * 1024)]
    pub disk_total: u32,

    /// Unique host identifier (auto-generated if not set).
    #[arg(long)]
    pub host_id: Option<String>,
}

impl Config {
    pub fn storage_kind(&self) -> Storage {
        self.storage.parse().unwrap_or(Storage::Raw)
    }

    /// Auto-detect CPU count if set to 0.
    pub fn effective_cpu(&self) -> u32 {
        if self.cpu_total == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(4)
        } else {
            self.cpu_total
        }
    }

    /// Auto-detect memory if set to 0 (read from /proc/meminfo).
    pub fn effective_mem(&self) -> u32 {
        if self.mem_total == 0 {
            read_total_mem_mb().unwrap_or(8192)
        } else {
            self.mem_total
        }
    }
}

/// Read total system memory in MB from /proc/meminfo.
fn read_total_mem_mb() -> Option<u32> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
            return Some((kb / 1024) as u32);
        }
    }
    None
}
