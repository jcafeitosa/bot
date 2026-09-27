use super::identity::OwnerId;

/// Product owner established via durable bootstrap (PG) — not human IdP auth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedProductOwner(OwnerId);

impl VerifiedProductOwner {
    pub fn new(owner: OwnerId) -> Self {
        Self(owner)
    }

    pub fn as_owner_id(&self) -> &OwnerId {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[cfg(test)]
    pub fn for_test(raw: &str) -> Self {
        Self(OwnerId::new(raw).expect("test owner id"))
    }
}
