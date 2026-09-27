use super::super::load::{env_nonempty, env_override_bool};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProductOwnerBootstrapConfig {
    pub bootstrap_owner_id: Option<String>,
    pub bootstrap_ack: bool,
}

impl ProductOwnerBootstrapConfig {
    pub fn from_env() -> Self {
        Self {
            bootstrap_owner_id: env_nonempty("BOT_PRODUCT_OWNER_BOOTSTRAP_ID"),
            bootstrap_ack: env_override_bool("BOT_PRODUCT_OWNER_BOOTSTRAP_ACK", false),
        }
    }

    /// Fail-closed: both non-empty owner id and explicit ACK are required to mutate PG.
    pub fn explicit_bootstrap_requested(&self) -> bool {
        self.bootstrap_owner_id.is_some() && self.bootstrap_ack
    }
}
