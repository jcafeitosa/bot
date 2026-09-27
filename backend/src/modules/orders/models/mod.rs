#![allow(dead_code)] // public seam; live wiring intentionally absent

mod error;
mod request;

pub use error::OrdersError;
pub use request::{OrderSide, SubmitOrderRequest};
