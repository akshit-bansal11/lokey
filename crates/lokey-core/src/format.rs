//! The vault file, `vault.lokey`, as JSON:
//!
//! ```text
//! header   format, version, kdf params, salt, public_key     (readable)
//! body     AES-256-GCM under the master key                   (sealed)
//!            secret_key, public_key copy, entries
//! inbox    HPKE records added by `set` without a password     (sealed to public_key)
//! ```
//!
//! The header is bound into the body's AES-GCM associated data, so editing a
//! salt or a cost parameter makes the body fail to open rather than weakening
//! it. The public key is deliberately not bound: a swapped key must not lock
//! the owner out. It is compared against the copy inside the body instead.

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{
    Error, Result,
    crypto::{KdfParams, Sealed},
};

const FORMAT: &str = "lokey";
/// v2 dropped the deletion-password check from the body. v1 files still open;
/// the first write re-seals them as v2, which older builds then refuse.
pub(crate) const VERSION: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultFile {
    pub format: String,
    pub version: u32,
    pub kdf: KdfParams,
    pub salt: String,
    pub public_key: String,
    pub body: Sealed,
    #[serde(default)]
    pub inbox: Vec<String>,
}

impl VaultFile {
    pub fn new(kdf: KdfParams, salt: String, public_key: String, body: Sealed) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            kdf,
            salt,
            public_key,
            body,
            inbox: Vec::new(),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let file: Self = serde_json::from_slice(bytes)
            .map_err(|err| Error::Corrupt(format!("vault file is not valid: {err}")))?;
        if file.format != FORMAT {
            return Err(Error::Corrupt("this file is not a lokey vault".into()));
        }
        if !(1..=VERSION).contains(&file.version) {
            return Err(Error::Corrupt(format!(
                "vault format v{} needs a newer lokey (this build reads v{VERSION})",
                file.version
            )));
        }
        file.kdf.check()?;
        Ok(file)
    }

    pub fn to_json(&self) -> Vec<u8> {
        // Only strings and integers: serialising cannot fail.
        serde_json::to_vec_pretty(self).expect("vault header serialises")
    }

    /// Associated data for the body: every header field an attacker could
    /// weaken, and nothing that changes on a password-less `set`.
    pub fn aad(&self) -> Vec<u8> {
        format!(
            "{}|{}|argon2id|{}|{}|{}|{}",
            self.format, self.version, self.kdf.m_kib, self.kdf.t, self.kdf.p, self.salt
        )
        .into_bytes()
    }
}

/// The decrypted body. Zeroed when dropped, since it holds every value.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Body {
    pub secret_key: String,
    pub public_key: String,
    /// A v1 body's `deletion` check is an unknown field now: skipped on read,
    /// gone on the next write.
    pub entries: Vec<Entry>,
}

#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Entry {
    pub project: String,
    pub key: String,
    pub value: String,
    /// Unix seconds of the last change.
    pub updated: u64,
}

/// Never prints the value, so an entry can be logged or debug-printed safely.
impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("project", &self.project)
            .field("key", &self.key)
            .field("value", &"<redacted>")
            .field("updated", &self.updated)
            .finish()
    }
}

/// One password-less `set`, as sealed into the inbox.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Record {
    pub project: String,
    pub key: String,
    pub value: String,
    pub at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> VaultFile {
        let body = Sealed {
            nonce: "bm9uY2U=".into(),
            ct: "Y3Q=".into(),
        };
        VaultFile::new(KdfParams::DEFAULT, "c2FsdA==".into(), "cGs=".into(), body)
    }

    #[test]
    fn parse_round_trips_to_json() {
        let file = sample();

        let parsed = VaultFile::parse(&file.to_json()).unwrap();

        assert_eq!(parsed.aad(), file.aad());
    }

    #[test]
    fn parse_rejects_other_format() {
        let mut file = sample();
        file.format = "secure-vault".into();

        let parsed = VaultFile::parse(&file.to_json());

        assert!(matches!(parsed, Err(Error::Corrupt(_))));
    }

    #[test]
    fn parse_rejects_a_newer_version() {
        let mut file = sample();
        file.version = VERSION + 1;

        let parsed = VaultFile::parse(&file.to_json());

        assert!(matches!(parsed, Err(Error::Corrupt(_))));
    }

    #[test]
    fn aad_changes_when_salt_changes() {
        let mut file = sample();
        let before = file.aad();
        file.salt = "b3RoZXI=".into();

        assert_ne!(file.aad(), before);
    }

    #[test]
    fn entry_debug_output_hides_value() {
        let entry = Entry {
            project: "web".into(),
            key: "API_KEY".into(),
            value: "sk-live-123".into(),
            updated: 0,
        };

        let printed = format!("{entry:?}");

        assert!(!printed.contains("sk-live-123"));
    }
}
