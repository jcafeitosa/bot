mod app;
mod backtest;
mod backtest_cli;
mod config;
mod domain;
mod error;
mod exchanges;
mod jev;
mod logging;
mod market;
mod market_feed;
mod persistence;
mod portfolio;
mod risk;
mod strategy;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;

#[derive(Debug, Subcommand)]
enum BotCommand {
    /// Run SMA crossover backtest on synthetic 1m candles
    Backtest(backtest_cli::BacktestCli),
}

#[derive(Debug, Parser)]
#[command(name = "bot", about = "Rust trading bot — terminal-only backend")]
struct TopCli {
    #[command(flatten)]
    monitor: config::MonitorCli,
    #[command(subcommand)]
    command: Option<BotCommand>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = TopCli::parse();
    match cli.command {
        Some(BotCommand::Backtest(args)) => backtest_cli::run(&args).await?,
        None => {
            let config = Config::load(&cli.monitor)?;
            let _logging_guard = logging::init(&config.logging)?;

            let database = if std::env::var("DATABASE_URL").is_ok() {
                match persistence::Database::connect_from_env().await {
                    Ok(db) => {
                        db.migrate().await?;
                        tracing::info!(target: "persistence", "PostgreSQL migrations applied");
                        Some(db)
                    }
                    Err(error) => {
                        tracing::warn!(target: "persistence", %error, "DATABASE_URL is set but persistence is unavailable");
                        None
                    }
                }
            } else {
                None
            };

            tracing::info!(
                target: "system",
                environment = %config.environment,
                operation = %config.operation,
                risk_profile = %config.risk_profile,
                symbol = %config.market.symbol,
                "Starting trading monitor"
            );

            if config.operation.is_hft() {
                anyhow::bail!("HFT is not supported by this REST polling implementation; use a dedicated low-latency feed/execution stack");
            }

            app::run(config, database).await?;
        }
    }
    Ok(())
}
