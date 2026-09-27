//! V18 (T-15 / W0-09): PostgreSQL isolado para persistência de mercado do monitor.
//! Comprova rollback transacional e replay idempotente de `persist_dataset`.

use super::{CandleRow, Database, MarketDatasetManifestRow, MarketDatasetPersistInput};

fn v18_dataset(dataset_id: &str) -> MarketDatasetPersistInput {
    let candles = [
        CandleRow {
            timestamp_ms: 1_700_000_000_000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1.0,
        },
        CandleRow {
            timestamp_ms: 1_700_000_060_000,
            open: 100.5,
            high: 102.0,
            low: 100.0,
            close: 101.0,
            volume: 2.0,
        },
    ];
    MarketDatasetPersistInput {
        manifest: MarketDatasetManifestRow {
            dataset_id: dataset_id.to_string(),
            symbol: "V18/TEST".to_string(),
            base_timeframe: "1m".to_string(),
            start_ms: candles[0].timestamp_ms,
            end_ms: candles[1].timestamp_ms,
            candle_count: candles.len(),
            gap_count: 0,
            source: "v18-pg-integration".to_string(),
        },
        candles: candles.to_vec(),
    }
}

async fn manifest_row_exists(db: &Database, dataset_id: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM market_datasets WHERE dataset_id = $1)",
    )
    .bind(dataset_id)
    .fetch_one(db.pool())
    .await
    .expect("manifest exists query")
}

async fn delete_v18_fixture(db: &Database, dataset_id: &str) {
    sqlx::query("DELETE FROM candles_1m WHERE dataset_id = $1")
        .bind(dataset_id)
        .execute(db.pool())
        .await
        .expect("delete candles");
    sqlx::query("DELETE FROM market_datasets WHERE dataset_id = $1")
        .bind(dataset_id)
        .execute(db.pool())
        .await
        .expect("delete manifest");
}

async fn insert_dataset_in_tx_then_rollback(db: &Database, input: &MarketDatasetPersistInput) {
    let mut tx = db.pool().begin().await.expect("begin tx");
    sqlx::query(
        "INSERT INTO market_datasets (dataset_id, symbol, base_timeframe, start_ms, end_ms, candle_count, gap_count, source) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (dataset_id) DO NOTHING",
    )
    .bind(&input.manifest.dataset_id)
    .bind(&input.manifest.symbol)
    .bind(&input.manifest.base_timeframe)
    .bind(input.manifest.start_ms)
    .bind(input.manifest.end_ms)
    .bind(input.manifest.candle_count as i64)
    .bind(input.manifest.gap_count as i64)
    .bind(&input.manifest.source)
    .execute(&mut *tx)
    .await
    .expect("insert manifest in tx");
    for candle in &input.candles {
        sqlx::query(
            "INSERT INTO candles_1m (symbol, time_ms, open, high, low, close, volume, dataset_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (symbol, time_ms) DO NOTHING",
        )
        .bind(&input.manifest.symbol)
        .bind(candle.timestamp_ms)
        .bind(candle.open)
        .bind(candle.high)
        .bind(candle.low)
        .bind(candle.close)
        .bind(candle.volume)
        .bind(&input.manifest.dataset_id)
        .execute(&mut *tx)
        .await
        .expect("insert candle in tx");
    }
    tx.rollback().await.expect("rollback tx");
}

#[tokio::test]
async fn pg_persist_dataset_transaction_rollback_and_idempotent_replay() {
    let Some(db) = super::pg_integration::database_for_integration_test().await else {
        return;
    };
    let dataset_id = format!(
        "v18-rollback-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    let input = v18_dataset(&dataset_id);
    let expected = input.manifest.candle_count as i64;

    insert_dataset_in_tx_then_rollback(&db, &input).await;
    assert_eq!(
        db.candle_count_for_dataset(&dataset_id)
            .await
            .expect("count after rollback"),
        0,
        "rolled-back tx must not leave candles"
    );
    assert!(
        !manifest_row_exists(&db, &dataset_id).await,
        "rolled-back tx must not leave market_datasets row"
    );

    db.persist_dataset(&input)
        .await
        .expect("first persist_dataset commit");
    assert_eq!(
        db.candle_count_for_dataset(&dataset_id)
            .await
            .expect("count after commit"),
        expected
    );

    db.persist_dataset(&input).await.expect("idempotent replay");
    assert_eq!(
        db.candle_count_for_dataset(&dataset_id)
            .await
            .expect("count after replay"),
        expected,
        "ON CONFLICT DO NOTHING replay must not duplicate candles"
    );

    delete_v18_fixture(&db, &dataset_id).await;
    assert_eq!(
        db.candle_count_for_dataset(&dataset_id)
            .await
            .expect("count after teardown"),
        0
    );
}

#[tokio::test]
async fn persist_dataset_rejects_conflicting_manifest_for_same_id() {
    use super::PersistenceError;

    let Some(db) = super::pg_integration::database_for_integration_test().await else {
        return;
    };
    let dataset_id = format!(
        "v18-manifest-conflict-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    let input = v18_dataset(&dataset_id);

    db.persist_dataset(&input).await.expect("initial persist");

    let mut conflicting = input.clone();
    conflicting.manifest.symbol = "V18/OTHER".to_string();

    let err = db
        .persist_dataset(&conflicting)
        .await
        .expect_err("conflicting manifest must fail");
    match &err {
        PersistenceError::DatasetManifestConflict { dataset_id: id } => {
            assert_eq!(id, &dataset_id);
        }
        other => panic!("unexpected error: {other:?}"),
    }

    delete_v18_fixture(&db, &dataset_id).await;
}
