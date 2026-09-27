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
}
