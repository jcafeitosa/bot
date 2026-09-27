use crate::modules::agents::VerifiedProductOwner;
use crate::presentation::http::admin_auth::HttpAdminAuth;
use crate::presentation::http::error::ApiError;

pub fn verify_register_owner_id(
    verified: Option<&VerifiedProductOwner>,
    http_admin: &HttpAdminAuth,
    owner_id: &str,
) -> Result<(), ApiError> {
    if let Some(verified) = verified {
        if owner_id.trim() != verified.as_str() {
            return Err(ApiError::owner_mismatch());
        }
    }
    http_admin.verify_register_owner_id(owner_id)
}

/// Fail-closed when PostgreSQL is wired: runtime promotion requires bootstrapped product owner.
pub fn verify_runtime_promotion_postgres_owner_bootstrap(
    postgres_connected: bool,
    verified: Option<&VerifiedProductOwner>,
) -> Result<(), ApiError> {
    if postgres_connected && verified.is_none() {
        return Err(ApiError::owner_bootstrap_required());
    }
    Ok(())
}

/// When product owner was bootstrapped in PostgreSQL, runtime promotion must trace to that owner.
/// Without agency bind, `promoted_by` must equal the verified owner id; with agency bind, pass the
/// promoting agent's `owner_id` after `assert_runtime_promotion_authorized`.
pub fn verify_promoted_by_product_owner(
    verified: Option<&VerifiedProductOwner>,
    promoted_by: &str,
    promoting_agent_owner_id: Option<&str>,
) -> Result<(), ApiError> {
    if let Some(verified) = verified {
        let matches = if let Some(owner) = promoting_agent_owner_id {
            owner.trim() == verified.as_str()
        } else {
            promoted_by.trim() == verified.as_str()
        };
        if !matches {
            return Err(ApiError::owner_mismatch());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::http::admin_auth::HttpAdminAuth;

    #[test]
    fn verified_owner_rejects_mismatch_before_http_bind() {
        let verified = VerifiedProductOwner::for_test("owner-bootstrapped");
        let auth = HttpAdminAuth::disabled();
        assert!(verify_register_owner_id(Some(&verified), &auth, "other").is_err());
        verify_register_owner_id(Some(&verified), &auth, "owner-bootstrapped").unwrap();
    }

    #[test]
    fn http_owner_bind_still_applies_when_verified_matches() {
        let verified = VerifiedProductOwner::for_test("owner-1");
        let auth = HttpAdminAuth::for_test_with_owner("t", "owner-1");
        verify_register_owner_id(Some(&verified), &auth, "owner-1").unwrap();
        assert!(verify_register_owner_id(Some(&verified), &auth, "owner-2").is_err());
    }

    #[test]
    fn promoted_by_direct_owner_match_when_no_agent_owner() {
        let verified = VerifiedProductOwner::for_test("owner-bootstrapped");
        verify_promoted_by_product_owner(Some(&verified), "owner-bootstrapped", None).unwrap();
        assert!(verify_promoted_by_product_owner(Some(&verified), "other", None).is_err());
    }

    #[test]
    fn promoted_by_agent_owner_must_match_verified() {
        let verified = VerifiedProductOwner::for_test("owner-bootstrapped");
        verify_promoted_by_product_owner(Some(&verified), "agent-1", Some("owner-bootstrapped"))
            .unwrap();
        assert!(
            verify_promoted_by_product_owner(Some(&verified), "agent-1", Some("other-owner"))
                .is_err()
        );
    }

    #[test]
    fn runtime_promotion_requires_owner_bootstrap_when_postgres_connected() {
        let verified = VerifiedProductOwner::for_test("owner-bootstrapped");
        assert!(verify_runtime_promotion_postgres_owner_bootstrap(true, None).is_err());
        verify_runtime_promotion_postgres_owner_bootstrap(true, Some(&verified)).unwrap();
        verify_runtime_promotion_postgres_owner_bootstrap(false, None).unwrap();
    }
}
