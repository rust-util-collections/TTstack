//! Controller configuration.

use clap::Parser;

/// TTstack central controller — fleet management and VM scheduling.
#[derive(Parser, Debug)]
#[command(name = "tt-ctl", version)]
pub struct Config {
    /// Listen address for the HTTP API.
    #[arg(long, default_value = "0.0.0.0:9200")]
    pub listen: String,

    /// Directory for persistent state (SQLite database).
    #[arg(long, default_value = "/home/ttstack/ctl")]
    pub data_dir: String,
}
