use crate::modules::bots::models::BotDefinition;

/// Future Gate 1: durable catalog of versioned bots (PostgreSQL).
pub trait BotCatalogStore {
    fn save_catalog(&mut self, entries: &[BotDefinition]) -> Result<(), String>;
    #[allow(dead_code)] // Gate 1 + unit tests in `modules/bots/tests.rs`
    fn load_catalog(&self) -> Result<Vec<BotDefinition>, String>;
}

#[derive(Debug, Default)]
pub struct NoopBotCatalogStore;

impl BotCatalogStore for NoopBotCatalogStore {
    fn save_catalog(&mut self, _entries: &[BotDefinition]) -> Result<(), String> {
        Ok(())
    }

    fn load_catalog(&self) -> Result<Vec<BotDefinition>, String> {
        Ok(Vec::new())
    }
}

/// In-process catalog snapshot until PostgreSQL Gate 1 lands.
#[cfg(test)]
#[derive(Debug, Default, Clone)]
pub struct InMemoryBotCatalogStore {
    entries: Vec<BotDefinition>,
}

#[cfg(test)]
impl InMemoryBotCatalogStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
impl BotCatalogStore for InMemoryBotCatalogStore {
    fn save_catalog(&mut self, entries: &[BotDefinition]) -> Result<(), String> {
        self.entries = entries.to_vec();
        Ok(())
    }

    fn load_catalog(&self) -> Result<Vec<BotDefinition>, String> {
        Ok(self.entries.clone())
    }
}
