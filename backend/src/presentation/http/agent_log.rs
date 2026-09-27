//! Session debug logging (NDJSON) for agent verification runs.

pub fn agent_debug_log(
    hypothesis_id: &str,
    location: &str,
    message: &str,
    data: serde_json::Value,
    run_id: &str,
) {
    // #region agent log
    use std::io::Write;
    let payload = serde_json::json!({
        "sessionId": "041845",
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "runId": run_id,
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/Users/jcafeitosa/Development/bot/.cursor/debug-041845.log")
    {
        let _ = writeln!(file, "{payload}");
    }
    // #endregion
}
