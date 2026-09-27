use crate::core::error::BotResult;
use crate::core::providers::{JevAdvisor, JevReviewInput};

/// Delegates advisory review to the shared Jev provider (`core::providers::jev`).
pub async fn delegate_jev_review(
    advisor: &JevAdvisor,
    input: &JevReviewInput,
) -> BotResult<Vec<String>> {
    advisor.review(input).await
}
