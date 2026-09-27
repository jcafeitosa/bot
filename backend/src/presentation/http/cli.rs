use std::net::SocketAddr;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct ServeCli {
    /// Socket address for the HTTP API (OpenAPI + Scalar UI).
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub bind: SocketAddr,
}
