#![allow(dead_code)]

use crate::modules::bots::models::BotDefinition;

/// Future Gate 1: durable catalog of versioned bots.
pub trait BotCatalogStore {
    fn save_catalog(&mut self, entries: &[BotDefinition]) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct NoopBotCatalogStore;

impl BotCatalogStore for NoopBotCatalogStore {
    fn save_catalog(&mut self, _entries: &[BotDefinition]) -> Result<(), String> {
        Ok(())
    }
}
