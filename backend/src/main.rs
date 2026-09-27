mod core;
mod modules;
mod presentation;

use crate::core::config::Config;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
enum BotCommand {
    /// Run SMA crossover backtest on synthetic 1m candles
    Backtest(modules::backtest::cli::BacktestCli),
    /// Start HTTP API with OpenAPI spec and Scalar UI at /docs
    Serve(presentation::http::ServeCli),
}

#[derive(Debug, Parser)]
#[command(
    name = "bot",
    about = "Rust trading bot — terminal UI and optional HTTP API"
)]
struct TopCli {
    #[command(flatten)]
    monitor: crate::core::config::MonitorCli,
    #[command(subcommand)]
    command: Option<BotCommand>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = TopCli::parse();
    match cli.command {
        Some(BotCommand::Backtest(args)) => modules::backtest::cli::run(&args).await?,
        Some(BotCommand::Serve(args)) => {
            let config = Config::load(&cli.monitor)?;
            let _logging_guard = crate::core::logging::init(&config.logging)?;
            presentation::http::run_server(args.bind, config).await?;
        }
        None => {
            let config = Config::load(&cli.monitor)?;
            let _logging_guard = crate::core::logging::init(&config.logging)?;
            if config.operation.is_hft() {
                anyhow::bail!("HFT is not supported by this REST polling implementation; use a dedicated low-latency feed/execution stack");
            }

            let persist_flag = std::env::var("PERSIST_MARKET_DATA")
                .map(Some)
                .or_else(|error| match error {
                    std::env::VarError::NotPresent => Ok(None),
                    std::env::VarError::NotUnicode(_) => {
                        Err(modules::monitor::StartupError::InvalidFlag)
                    }
                })?;
            let database = modules::monitor::bootstrap_monitor(
                persist_flag.as_deref(),
                &config.market.timeframe,
                || match std::env::var("DATABASE_URL") {
                    Ok(url) => Ok(Some(url)),
                    Err(std::env::VarError::NotPresent) => Ok(None),
                    Err(std::env::VarError::NotUnicode(_)) => {
                        Err(modules::monitor::StartupError::InvalidUrl)
                    }
                },
                modules::monitor::connect_database,
            )
            .await?;
            if database.is_some() {
                tracing::info!(target: "persistence", "PostgreSQL migrations applied");
            }

            tracing::info!(
                target: "system",
                environment = %config.environment,
                operation = %config.operation,
                risk_profile = %config.risk_profile,
                symbol = %config.market.symbol,
                "Starting trading monitor"
            );

            modules::monitor::run(config, database).await?;
        }
    }
    Ok(())
}
