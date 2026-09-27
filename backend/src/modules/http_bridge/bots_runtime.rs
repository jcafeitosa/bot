use crate::modules::bots::{
    BotPromotionRecord, BotRuntimePort, BotRuntimeStatus, BotsError, PromoteBotRequest,
};

pub fn bot_runtime_status(runtime: &dyn BotRuntimePort) -> BotRuntimeStatus {
    runtime.status()
}

pub fn promote_bot(
    runtime: &dyn BotRuntimePort,
    request: PromoteBotRequest,
) -> Result<BotPromotionRecord, BotsError> {
    runtime.promote(request)
}

pub fn demote_bot(runtime: &dyn BotRuntimePort) -> Result<(), BotsError> {
    runtime.demote()
}
