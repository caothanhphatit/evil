use std::fmt::Write;

use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SessionTokenHash([u8; 32]);

impl SessionTokenHash {
    pub fn from_token(token: Uuid) -> Self {
        Self(Sha256::digest(token.as_bytes()).into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn cache_key_suffix(&self) -> String {
        let mut encoded = String::with_capacity(64);
        for byte in self.0 {
            write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hash_is_stable_and_does_not_expose_the_cookie_value() {
        let token = Uuid::from_u128(1);
        let digest = SessionTokenHash::from_token(token);

        assert_eq!(digest, SessionTokenHash::from_token(token));
        assert_eq!(digest.as_bytes().len(), 32);
        assert_eq!(digest.cache_key_suffix().len(), 64);
        assert!(!digest.cache_key_suffix().contains(&token.to_string()));
    }
}
