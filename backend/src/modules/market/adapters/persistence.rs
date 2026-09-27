use crate::core::persistence::{Database, PersistenceError};
use crate::modules::market::models::HistoricalDataset;

pub async fn persist_historical_dataset(
    db: &Database,
    dataset: &HistoricalDataset,
) -> Result<(), PersistenceError> {
    db.persist_dataset(&dataset.persist_input()).await
}
