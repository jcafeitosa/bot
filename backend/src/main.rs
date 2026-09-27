mod core;
mod modules;
mod presentation;

use std::path::Path;

use crate::core::config::{load, Config, MonitorEnvError};
use crate::modules::monitor::StartupError;
use anyhow::Result;

fn map_monitor_env(error: MonitorEnvError) -> StartupError {
    match error {
        MonitorEnvError::InvalidFlag => StartupError::InvalidFlag,
        MonitorEnvError::InvalidUrl => StartupError::InvalidUrl,
    }
}

fn read_persist_flag() -> Result<Option<String>, StartupError> {
    crate::core::config::persist_market_data_flag_raw().map_err(map_monitor_env)
}

use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
enum BotCommand {
    /// Run SMA crossover backtest on synthetic 1m candles
    Backtest(modules::backtest::cli::BacktestCli),
    /// Start HTTP API with OpenAPI spec and Scalar UI at /docs
    Serve(presentation::http::ServeCli),
    /// Graph projection operations (Neo4j outbox drain)
    GraphProjection(crate::core::database::GraphProjectionCli),
    /// Read-only Neo4j graph queries (F3)
    Graph(crate::core::database::GraphCli),
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

fn bot_config_path_for_cli(cli: &TopCli) -> std::path::PathBuf {
    match &cli.command {
        Some(BotCommand::Backtest(args)) => args
            .config
            .clone()
            .unwrap_or_else(|| cli.monitor.config.clone()),
        Some(BotCommand::Serve(args)) => args
            .config
            .clone()
            .unwrap_or_else(|| cli.monitor.config.clone()),
        Some(BotCommand::GraphProjection(_)) | Some(BotCommand::Graph(_)) => {
            cli.monitor.config.clone()
        }
        None => cli.monitor.config.clone(),
    }
}

fn ensure_bot_config_readable(path: &Path) -> Result<()> {
    load::read_bot_config_file(path)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    crate::core::config::ensure_dotenv_loaded();
    let cli = TopCli::parse();
    crate::core::config::SystemConfig::init_from_path(&cli.monitor.system_config)?;
    match &cli.command {
        Some(BotCommand::Backtest(_)) | None => {
            ensure_bot_config_readable(&bot_config_path_for_cli(&cli))?;
        }
        Some(BotCommand::Serve(_))
        | Some(BotCommand::GraphProjection(_))
        | Some(BotCommand::Graph(_)) => {}
    }
    match cli.command {
        Some(BotCommand::Backtest(args)) => {
            modules::backtest::cli::run(&args, &cli.monitor).await?
        }
        Some(BotCommand::GraphProjection(args)) => {
            crate::core::database::graph_projection_cli::run(&args).await?;
        }
        Some(BotCommand::Graph(args)) => {
            crate::core::database::graph_cli::run(&args).await?;
        }
        Some(BotCommand::Serve(args)) => {
            let mut monitor_cli = cli.monitor.clone();
            if let Some(path) = args.config {
                monitor_cli.config = path;
            }
            let config = Config::load(&monitor_cli)?;
            let _logging_guard = crate::core::logging::init(&config.logging)?;
            let monitor_handle = if args.with_monitor {
                if config.operation.is_hft() {
                    anyhow::bail!(
                        "HFT is not supported by this REST polling implementation; omit --with-monitor or use a non-HFT operation mode"
                    );
                }
                let persist_flag = read_persist_flag()?;
                let database = crate::core::database::AppDatabases::bootstrap_monitor_postgres(
                    persist_flag.as_deref(),
                    &config.market.timeframe,
                )
                .await?;
                let agent_hook = modules::agents::monitor_agent_hook_from_env();
                let (handle, task) =
                    modules::monitor::spawn_headless_for_api(config.clone(), database, agent_hook)
                        .await?;
                tokio::spawn(async move {
                    match task.await {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            tracing::error!(target: "monitor", %error, "headless monitor stopped with error")
                        }
                        Err(error) => {
                            tracing::error!(target: "monitor", %error, "headless monitor task failed")
                        }
                    }
                });
                Some(handle)
            } else {
                None
            };
            presentation::http::run_server(args.bind, config, monitor_handle).await?;
        }
        None => {
            let config = Config::load(&cli.monitor)?;
            let _logging_guard = crate::core::logging::init(&config.logging)?;
            if config.operation.is_hft() {
                anyhow::bail!("HFT is not supported by this REST polling implementation; use a dedicated low-latency feed/execution stack");
            }

            let persist_flag = read_persist_flag()?;
            let database = crate::core::database::AppDatabases::bootstrap_monitor_postgres(
                persist_flag.as_deref(),
                &config.market.timeframe,
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

            let agent_hook = modules::agents::monitor_agent_hook_from_env();
            modules::monitor::run_with_agent_hook(config, database, agent_hook).await?;
        }
    }
    Ok(())
}
