use super::super::database::postgres_url_from_env;
use crate::core::error::BotError;

pub fn require_database_url_for_persist() -> Result<(), BotError> {
    match postgres_url_from_env() {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(BotError::Configuration(
            "--persist requires DATABASE_URL (postgresql://…/trading_bot)".into(),
        )),
        Err(error) => Err(BotError::Configuration(error.to_string())),
    }
}
