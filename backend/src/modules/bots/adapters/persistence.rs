use crate::modules::bots::models::BotDefinition;

/// Durable catalog of versioned bots (in-memory or PostgreSQL Gate 1).
#[async_trait::async_trait]
pub trait BotCatalogStore {
    async fn save_catalog(&mut self, entries: &[BotDefinition]) -> Result<(), String>;
    async fn load_catalog(&self) -> Result<Vec<BotDefinition>, String>;
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct NoopBotCatalogStore;

#[async_trait::async_trait]
impl BotCatalogStore for NoopBotCatalogStore {
    async fn save_catalog(&mut self, _entries: &[BotDefinition]) -> Result<(), String> {
        Ok(())
    }

    async fn load_catalog(&self) -> Result<Vec<BotDefinition>, String> {
        Ok(Vec::new())
    }
}

/// In-process catalog snapshot when PostgreSQL is unavailable.
#[derive(Debug, Default, Clone)]
pub struct InMemoryBotCatalogStore {
    entries: Vec<BotDefinition>,
}

impl InMemoryBotCatalogStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl BotCatalogStore for InMemoryBotCatalogStore {
    async fn save_catalog(&mut self, entries: &[BotDefinition]) -> Result<(), String> {
        self.entries = entries.to_vec();
        Ok(())
    }

    async fn load_catalog(&self) -> Result<Vec<BotDefinition>, String> {
        Ok(self.entries.clone())
    }
}
