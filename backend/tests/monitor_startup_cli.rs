use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn run_rejected_monitor(flag: &str, url: Option<&str>) -> String {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("monitor.toml");
    let config = include_str!("../src/core/config/bot.toml")
        .replace("operation = \"day_trader\"", "operation = \"scalper\"")
        .replace("timeframe = \"15m\"", "timeframe = \"1m\"");
    fs::write(&config_path, config).unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_bot"));
    command
        .arg("--config")
        .arg(&config_path)
        .current_dir(dir.path())
        .env("PERSIST_MARKET_DATA", flag)
        .env_remove("DATABASE_URL")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(url) = url {
        command.env("DATABASE_URL", url);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("monitor did not reject invalid startup before market/TUI");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains("Starting trading monitor"));
    assert!(!combined.contains("Exchange registry loaded"));
    combined
}

#[test]
fn invalid_flag_fails_at_cli_entry_before_market_start() {
    let output = run_rejected_monitor("yes", Some("private-url"));
    assert!(output.contains("PERSIST_MARKET_DATA must be 0, 1, false, or true"));
    assert!(!output.contains("private-url"));
}

#[test]
fn required_without_url_fails_at_cli_entry_before_market_start() {
    let output = run_rejected_monitor("1", None);
    assert!(output.contains("DATABASE_URL is required"));
}
