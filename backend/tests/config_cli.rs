use std::{
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn assert_config_error_reports_path(command: &[&str], working_dir: &Path, expected_path: &str) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bot"))
        .args(command)
        .current_dir(working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("bot binary should start");
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("command did not reject missing config: {command:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded: {command:?}"
    );
    assert!(
        stderr.contains(expected_path),
        "missing config path was not reported: {stderr}"
    );
}

#[test]
fn monitor_and_backtest_reject_missing_default_and_explicit_config() {
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("missing-bot.toml");
    let explicit = absent.to_str().unwrap();
    let cases: &[&[&str]] = &[
        &[],
        &["--config", explicit],
        &["backtest"],
        &["backtest", "--config", explicit],
    ];
    for command in cases {
        let expected_path = if command.contains(&"--config") {
            explicit
        } else {
            "src/core/config/bot.toml"
        };
        assert_config_error_reports_path(command, dir.path(), expected_path);
    }
}

#[test]
fn monitor_and_backtest_reject_unreadable_config_path() {
    let dir = tempfile::tempdir().unwrap();
    let directory_path = dir.path().to_str().unwrap();
    assert_config_error_reports_path(&["--config", directory_path], dir.path(), directory_path);
    assert_config_error_reports_path(
        &["backtest", "--config", directory_path],
        dir.path(),
        directory_path,
    );
}
