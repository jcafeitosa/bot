use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct ServeCli {
    /// Socket address for the HTTP API (OpenAPI + Scalar UI).
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub bind: SocketAddr,
    /// Override bot.toml path for this API process (same as global `--config`).
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Run the live market monitor in-process (headless) so `/api/v1/monitor/*` routes work.
    #[arg(long)]
    pub with_monitor: bool,
}
