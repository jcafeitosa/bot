use super::super::load::env_nonempty;

#[derive(Clone, Debug, Default)]
pub struct HttpAdminAuthConfig {
    pub token: Option<String>,
    pub bound_owner_id: Option<String>,
    pub bound_agency_id: Option<String>,
}

impl HttpAdminAuthConfig {
    pub fn from_env() -> Self {
        Self {
            token: env_nonempty("BOT_HTTP_ADMIN_TOKEN"),
            bound_owner_id: env_nonempty("BOT_HTTP_OWNER_ID"),
            bound_agency_id: env_nonempty("BOT_HTTP_AGENCY_ID"),
        }
    }
}
