//! Order submission seam — risk validation then fail-closed execution port.
//!
//! Live trading remains disabled; see `modules/exchanges/rest::authorize_rest_use`.

#![allow(dead_code)] // public foundation seam; live wiring intentionally absent
#![allow(unused_imports)]

pub mod adapters;
pub mod controllers;
pub mod models;

pub use adapters::{FailClosedExecutor, OrderExecutionPort};
pub use controllers::submit_order;
pub use models::{OrderSide, OrdersError, SubmitOrderRequest};

#[cfg(test)]
mod tests;
