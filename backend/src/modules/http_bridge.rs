//! HTTP-facing facades so `presentation` does not import domain modules directly.

pub mod risk {
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    use crate::core::error::BotError;
    use crate::modules::config_api::RiskProfile;
    use crate::modules::risk::{
        controllers::{profile_limits, validate_intent},
        models::{OrderIntent, RiskLimits},
    };

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct RiskLimitsBody {
        pub max_order_quote: f64,
        pub max_daily_loss_quote: f64,
        pub max_open_positions: usize,
    }

    #[derive(Debug, Deserialize, ToSchema)]
    pub struct ProfileLimitsRequest {
        pub profile: RiskProfile,
        pub base: RiskLimitsBody,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct ProfileLimitsResponse {
        pub limits: RiskLimitsBody,
    }

    #[derive(Debug, Deserialize, ToSchema)]
    pub struct ValidateIntentRequest {
        pub intent: OrderIntentBody,
        pub limits: RiskLimitsBody,
        #[serde(default)]
        pub is_close: bool,
    }

    #[derive(Debug, Deserialize, ToSchema)]
    pub struct OrderIntentBody {
        pub quote_amount: f64,
        pub estimated_daily_loss: f64,
        pub open_positions: usize,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct ValidateIntentResponse {
        pub accepted: bool,
    }

    impl From<RiskLimitsBody> for RiskLimits {
        fn from(value: RiskLimitsBody) -> Self {
            Self {
                max_order_quote: value.max_order_quote,
                max_daily_loss_quote: value.max_daily_loss_quote,
                max_open_positions: value.max_open_positions,
            }
        }
    }

    impl From<RiskLimits> for RiskLimitsBody {
        fn from(value: RiskLimits) -> Self {
            Self {
                max_order_quote: value.max_order_quote,
                max_daily_loss_quote: value.max_daily_loss_quote,
                max_open_positions: value.max_open_positions,
            }
        }
    }

    impl From<OrderIntentBody> for OrderIntent {
        fn from(value: OrderIntentBody) -> Self {
            Self {
                quote_amount: value.quote_amount,
                estimated_daily_loss: value.estimated_daily_loss,
                open_positions: value.open_positions,
            }
        }
    }

    pub fn compute_profile_limits(body: ProfileLimitsRequest) -> ProfileLimitsResponse {
        let limits = profile_limits(body.profile, body.base.into());
        ProfileLimitsResponse {
            limits: limits.into(),
        }
    }

    pub fn validate_order_intent(
        body: ValidateIntentRequest,
    ) -> Result<ValidateIntentResponse, BotError> {
        validate_intent(body.intent.into(), body.limits.into(), body.is_close)?;
        Ok(ValidateIntentResponse { accepted: true })
    }
}

pub mod strategy {
    use serde::{Deserialize, Serialize};
    use utoipa::{IntoParams, ToSchema};

    use crate::modules::config_api::OperationMode;
    use crate::modules::strategy::periods_for_mode;

    #[derive(Debug, Deserialize, IntoParams, ToSchema)]
    pub struct PeriodsQuery {
        pub operation: OperationMode,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct StrategyPeriodsResponse {
        pub operation: OperationMode,
        pub fast_period: usize,
        pub slow_period: usize,
    }

    pub fn sma_periods(query: PeriodsQuery) -> StrategyPeriodsResponse {
        let (fast_period, slow_period) = periods_for_mode(query.operation);
        StrategyPeriodsResponse {
            operation: query.operation,
            fast_period,
            slow_period,
        }
    }
}
