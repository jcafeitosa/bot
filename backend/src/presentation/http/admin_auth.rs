//! Optional fail-closed admin bearer for mutating HTTP routes (dev/staging seam; not full owner auth).

use axum::http::{header, HeaderMap};

use super::error::ApiError;

#[derive(Clone, Debug, Default)]
pub struct HttpAdminAuth {
    token: Option<String>,
    bound_owner_id: Option<String>,
    bound_agency_id: Option<String>,
}

impl HttpAdminAuth {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn from_env() -> Self {
        let token = match std::env::var("BOT_HTTP_ADMIN_TOKEN") {
            Ok(raw) if !raw.trim().is_empty() => Some(raw.trim().to_string()),
            _ => None,
        };
        let bound_owner_id = match std::env::var("BOT_HTTP_OWNER_ID") {
            Ok(raw) if !raw.trim().is_empty() => Some(raw.trim().to_string()),
            _ => None,
        };
        let bound_agency_id = match std::env::var("BOT_HTTP_AGENCY_ID") {
            Ok(raw) if !raw.trim().is_empty() => Some(raw.trim().to_string()),
            _ => None,
        };
        Self {
            token,
            bound_owner_id,
            bound_agency_id,
        }
    }

    #[cfg(test)]
    pub fn for_test(token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            bound_owner_id: None,
            bound_agency_id: None,
        }
    }

    #[cfg(test)]
    pub fn for_test_with_owner(token: impl Into<String>, owner_id: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            bound_owner_id: Some(owner_id.into()),
            bound_agency_id: None,
        }
    }

    #[cfg(test)]
    pub fn for_test_with_agency(token: impl Into<String>, agency_id: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            bound_owner_id: None,
            bound_agency_id: Some(agency_id.into()),
        }
    }

    #[cfg(test)]
    pub fn for_test_bound_agency(agency_id: impl Into<String>) -> Self {
        Self {
            token: None,
            bound_owner_id: None,
            bound_agency_id: Some(agency_id.into()),
        }
    }

    pub fn bound_agency_id_opt(&self) -> Option<&str> {
        self.bound_agency_id.as_deref()
    }

    pub fn enabled(&self) -> bool {
        self.token.is_some()
    }

    pub fn owner_binding_active(&self) -> bool {
        self.bound_owner_id.is_some()
    }

    pub fn agency_binding_active(&self) -> bool {
        self.bound_agency_id.is_some()
    }

    pub fn verify_headers(&self, headers: &HeaderMap) -> Result<(), ApiError> {
        if self.token.is_none() {
            return Ok(());
        }
        let expected = self.token.as_deref().expect("enabled implies token");
        let provided = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_bearer_token)
            .ok_or_else(ApiError::unauthorized)?;
        if constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
            Ok(())
        } else {
            Err(ApiError::unauthorized())
        }
    }

    /// When `BOT_HTTP_OWNER_ID` is set together with admin token, agent registration must use that owner id.
    pub fn verify_register_owner_id(&self, owner_id: &str) -> Result<(), ApiError> {
        if let Some(expected) = &self.bound_owner_id {
            if owner_id.trim() != expected {
                return Err(ApiError::owner_mismatch());
            }
        }
        Ok(())
    }

    /// When `BOT_HTTP_AGENCY_ID` is set, agent routes must target that agency.
    pub fn verify_agency_id(&self, agency_id: &str) -> Result<(), ApiError> {
        if let Some(expected) = &self.bound_agency_id {
            if agency_id.trim() != expected {
                return Err(ApiError::http_agency_mismatch());
            }
        }
        Ok(())
    }
}

fn parse_bearer_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let prefix = "Bearer ";
    if !trimmed.starts_with(prefix) {
        return None;
    }
    let token = trimmed[prefix.len()..].trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= *a ^ *b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_auth_allows_missing_header() {
        let auth = HttpAdminAuth::disabled();
        assert!(!auth.enabled());
        auth.verify_headers(&HeaderMap::new()).unwrap();
    }

    #[test]
    fn enabled_auth_requires_bearer() {
        let auth = HttpAdminAuth::for_test("secret-token");
        assert!(auth.verify_headers(&HeaderMap::new()).is_err());
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer secret-token".parse().expect("header"),
        );
        auth.verify_headers(&headers).unwrap();
    }

    #[test]
    fn bound_owner_id_rejects_mismatch_on_register() {
        let auth = HttpAdminAuth::for_test_with_owner("t", "owner-allowed");
        assert!(auth.verify_register_owner_id("other").is_err());
        auth.verify_register_owner_id("owner-allowed").unwrap();
    }

    #[test]
    fn bound_agency_id_rejects_mismatch() {
        let auth = HttpAdminAuth::for_test_with_agency("t", "acme-only");
        assert!(auth.verify_agency_id("other").is_err());
        auth.verify_agency_id("acme-only").unwrap();
    }

    #[test]
    fn binding_active_flags_reflect_env_bindings_without_leaking_ids() {
        let disabled = HttpAdminAuth::disabled();
        assert!(!disabled.owner_binding_active());
        assert!(!disabled.agency_binding_active());
        let owner = HttpAdminAuth::for_test_with_owner("t", "owner-1");
        assert!(owner.owner_binding_active());
        assert!(!owner.agency_binding_active());
        let agency = HttpAdminAuth::for_test_with_agency("t", "agency-1");
        assert!(!agency.owner_binding_active());
        assert!(agency.agency_binding_active());
    }
}
