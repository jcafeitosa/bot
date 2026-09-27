use super::persistence::{BotCatalogStore, InMemoryBotCatalogStore};
use super::pg_catalog::PgBotCatalogStore;
use crate::core::database::AppDatabases;
use crate::modules::bots::models::BotDefinition;

pub enum BotCatalogBackend {
    Memory(InMemoryBotCatalogStore),
    Postgres(PgBotCatalogStore),
}

impl BotCatalogBackend {
    pub fn from_databases(databases: &AppDatabases) -> Self {
        if let Some(db) = databases.postgres_handle() {
            Self::Postgres(PgBotCatalogStore::new(db.as_postgres()))
        } else {
            Self::Memory(InMemoryBotCatalogStore::new())
        }
    }
}

#[async_trait::async_trait]
impl BotCatalogStore for BotCatalogBackend {
    async fn save_catalog(&mut self, entries: &[BotDefinition]) -> Result<(), String> {
        match self {
            Self::Memory(store) => store.save_catalog(entries).await,
            Self::Postgres(store) => store.save_catalog(entries).await,
        }
    }

    async fn load_catalog(&self) -> Result<Vec<BotDefinition>, String> {
        match self {
            Self::Memory(store) => store.load_catalog().await,
            Self::Postgres(store) => store.load_catalog().await,
        }
    }
}
