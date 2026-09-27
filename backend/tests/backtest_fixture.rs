use std::{fs, process::Command};

#[test]
fn each_supported_preset_and_timeframe_closes_a_signal_trade() {
    let template = include_str!("../src/core/config/bot.toml");
    let cases = [
        ("scalper", "1m", 5, 20),
        ("scalper", "3m", 5, 20),
        ("scalper", "5m", 5, 20),
        ("day_trader", "5m", 5, 20),
        ("day_trader", "15m", 5, 20),
        ("day_trader", "30m", 5, 20),
        ("swing_trader", "1h", 20, 50),
        ("swing_trader", "4h", 20, 50),
    ];
    let dir = tempfile::tempdir().unwrap();
    for (mode, timeframe, fast, slow) in cases {
        let config = template
            .replace(
                "operation = \"day_trader\"",
                &format!("operation = \"{mode}\""),
            )
            .replace(
                "timeframe = \"15m\"",
                &format!("timeframe = \"{timeframe}\""),
            )
            .replace("sma_fast = 5", &format!("sma_fast = {fast}"))
            .replace("sma_slow = 20", &format!("sma_slow = {slow}"));
        let path = dir.path().join("bot.toml");
        fs::write(&path, config).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_bot"))
            .args(["backtest", "--config"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode} {timeframe}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let trades = summary["trades"].as_u64().unwrap();
        let wins = summary["wins"].as_u64().unwrap();
        let losses = summary["losses"].as_u64().unwrap();
        assert!(trades >= 1, "{mode} {timeframe}: {summary}");
        assert_eq!(wins + losses, trades, "{mode} {timeframe}: {summary}");
        assert_eq!(
            summary["timeframe_minutes"],
            match timeframe {
                "1m" => 1,
                "3m" => 3,
                "5m" => 5,
                "15m" => 15,
                "30m" => 30,
                "1h" => 60,
                "4h" => 240,
                _ => unreachable!(),
            }
        );
    }
}
