#![allow(dead_code)] // public seam; live wiring intentionally absent

mod execution_port;
mod fail_closed;

pub use execution_port::OrderExecutionPort;
pub use fail_closed::FailClosedExecutor;
