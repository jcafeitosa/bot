#![allow(dead_code)]
//! HTTP-facing facades so `presentation` does not import domain modules directly.

pub mod agents;
pub mod application;
pub mod backtest;
pub mod bots;
pub mod config;
pub mod exchanges;
pub mod monitor;
pub mod orders;
pub mod portfolio;
pub mod providers;
pub mod risk;
pub mod strategy;
