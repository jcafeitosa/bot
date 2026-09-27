use sqlx::{PgPool, Row};

const SINGLETON_ID: i16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorSupervisorSnapshotRow {
    pub last_tick_ms: i64,
    pub promoted_bot_id: Option<String>,
    pub updated_at_ms: i64,
}

pub async fn load_monitor_supervisor_snapshot(
    pool: &PgPool,
) -> Result<Option<MonitorSupervisorSnapshotRow>, String> {
    let row = sqlx::query(
        "SELECT last_tick_ms, promoted_bot_id, updated_at_ms \
         FROM monitor_supervisor_snapshot WHERE singleton_id = $1",
    )
    .bind(SINGLETON_ID)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|record| MonitorSupervisorSnapshotRow {
        last_tick_ms: record.get("last_tick_ms"),
        promoted_bot_id: record.get("promoted_bot_id"),
        updated_at_ms: record.get("updated_at_ms"),
    }))
}

pub async fn save_monitor_supervisor_snapshot_best_effort(
    pool: &PgPool,
    last_tick_ms: i64,
    promoted_bot_id: Option<&str>,
    updated_at_ms: i64,
) -> Result<(), String> {
    sqlx::query(
        r#"INSERT INTO monitor_supervisor_snapshot (singleton_id, last_tick_ms, promoted_bot_id, updated_at_ms)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (singleton_id) DO UPDATE SET
             last_tick_ms = EXCLUDED.last_tick_ms,
             promoted_bot_id = EXCLUDED.promoted_bot_id,
             updated_at_ms = EXCLUDED.updated_at_ms"#,
    )
    .bind(SINGLETON_ID)
    .bind(last_tick_ms)
    .bind(promoted_bot_id)
    .bind(updated_at_ms)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_sql_declares_monitor_supervisor_snapshot_table() {
        let sql = include_str!("migrations/0011_monitor_supervisor_snapshot.sql");
        assert!(sql.contains("monitor_supervisor_snapshot"));
        assert!(sql.contains("last_tick_ms"));
        assert!(sql.contains("promoted_bot_id"));
        assert!(sql.contains("updated_at_ms"));
    }

    #[test]
    fn snapshot_row_equality_is_fieldwise() {
        let a = MonitorSupervisorSnapshotRow {
            last_tick_ms: 1_700_000_000_000,
            promoted_bot_id: Some("bot-a@1".into()),
            updated_at_ms: 99,
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(
            a,
            MonitorSupervisorSnapshotRow {
                last_tick_ms: a.last_tick_ms,
                promoted_bot_id: None,
                updated_at_ms: a.updated_at_ms,
            }
        );
    }

    #[tokio::test]
    async fn pg_monitor_supervisor_snapshot_round_trip() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let pool = db.pool();
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let bot_id = format!("monitor-snap-{suffix}");
        let tick = 1_234_567_890_000_i64;
        let updated = 42_i64;
        save_monitor_supervisor_snapshot_best_effort(pool, tick, Some(&bot_id), updated)
            .await
            .expect("save");
        let loaded = load_monitor_supervisor_snapshot(pool)
            .await
            .expect("load")
            .expect("row");
        assert_eq!(
            loaded,
            MonitorSupervisorSnapshotRow {
                last_tick_ms: tick,
                promoted_bot_id: Some(bot_id),
                updated_at_ms: updated,
            }
        );
        save_monitor_supervisor_snapshot_best_effort(pool, tick + 60_000, None, updated + 1)
            .await
            .expect("upsert clear bot");
        let cleared = load_monitor_supervisor_snapshot(pool)
            .await
            .expect("load2")
            .expect("row2");
        assert_eq!(cleared.promoted_bot_id, None);
        assert_eq!(cleared.last_tick_ms, tick + 60_000);
    }
}
