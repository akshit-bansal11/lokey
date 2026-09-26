//! The vault file, `vault.lokey`, as JSON. Version 3:
//!
//! ```text
//! header    format, version, kdf params, salt                      (readable)
//! key       the data key, sealed under the master password's key   (sealed)
//! recovery  optional: a salt and the data key, sealed under the
//!           recovery key's key                                      (sealed)
//! body      AES-256-GCM under the data key: the entries             (sealed)
//! ```
//!
//! Every header field an attacker could weaken is bound into the associated
//! data of the sealed key, so editing a salt or a cost parameter makes the
//! vault fail to open rather than weakening it. The data key is random and is
//! replaced whenever the password or the recovery key changes, so an old
//! password or recovery key, together with an old copy of the file, opens
//! only that old copy.
//!
//! Versions 1 and 2 sealed the body directly under the password's key and
//! carried a sealed inbox; they are read and rewritten as v3 on unlock.

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{
    Error, Result,
    crypto::{KdfParams, Sealed},
};

const FORMAT: &str = "lokey";
/// v2 dropped the deletion-password check from the body. v3 added the random
/// data key and the recovery key, and dropped the inbox.
pub(crate) const VERSION: u32 = 3;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultFile {
    pub format: String,
    pub version: u32,
    pub kdf: KdfParams,
    pub salt: String,
    /// v3: the data key, sealed under the master password's key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<Sealed>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<Recovery>,
    pub body: Sealed,
    /// v1/v2 only: the key the removed command line sealed inbox records to.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub public_key: String,
    /// v1/v2 only: values added without the password, merged on upgrade.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inbox: Vec<String>,
}

/// The data key sealed under a key derived from the recovery key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recovery {
    pub salt: String,
    pub key: Sealed,
}

impl VaultFile {
    pub fn new(kdf: KdfParams) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            kdf,
            salt: String::new(),
            key: None,
            recovery: None,
            body: Sealed::default(),
            public_key: String::new(),
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
        if !file.is_legacy() && file.key.is_none() {
            return Err(Error::Corrupt("vault file has no data key".into()));
        }
        file.kdf.check()?;
        Ok(file)
    }

    pub fn to_json(&self) -> Vec<u8> {
        // Only strings and integers: serialising cannot fail.
        serde_json::to_vec_pretty(self).expect("vault header serialises")
    }

    /// A v1 or v2 file, whose body is sealed directly under the password's key.
    pub fn is_legacy(&self) -> bool {
        self.version < 3
    }

    /// Associated data for the data key sealed under the password's key:
    /// every header field an attacker could weaken.
    pub fn password_aad(&self) -> Vec<u8> {
        self.aad_for("password", &self.salt)
    }

    /// Associated data for the data key sealed under the recovery key's key.
    pub fn recovery_aad(&self, salt: &str) -> Vec<u8> {
        self.aad_for("recovery", salt)
    }

    /// Associated data for the body. The version is bound, so a v3 body can
    /// never be read as an older format.
    pub fn body_aad(&self) -> Vec<u8> {
        format!("{}|{}|body", self.format, self.version).into_bytes()
    }

    /// Associated data of a v1/v2 body.
    pub fn legacy_aad(&self) -> Vec<u8> {
        format!(
            "{}|{}|argon2id|{}|{}|{}|{}",
            self.format, self.version, self.kdf.m_kib, self.kdf.t, self.kdf.p, self.salt
        )
        .into_bytes()
    }

    fn aad_for(&self, slot: &str, salt: &str) -> Vec<u8> {
        format!(
            "{}|{}|{slot}|argon2id|{}|{}|{}|{salt}",
            self.format, self.version, self.kdf.m_kib, self.kdf.t, self.kdf.p
        )
        .into_bytes()
    }
}

/// The decrypted body. Zeroed when dropped, since it holds every value.
#[derive(Default, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Body {
    /// v1/v2 only: the secret key that opens the inbox. Read, never written.
    /// A v1 body's `deletion` check is an unknown field: skipped on read.
    #[serde(default, rename = "secret_key", skip_serializing)]
    pub inbox_secret: String,
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

/// One value added through the removed command line, as sealed into a v1/v2
/// inbox.
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
        let mut file = VaultFile::new(KdfParams::DEFAULT);
        file.salt = "c2FsdA==".into();
        file.key = Some(Sealed {
            nonce: "bm9uY2U=".into(),
            ct: "a2V5".into(),
        });
        file.body = Sealed {
            nonce: "bm9uY2U=".into(),
            ct: "Y3Q=".into(),
        };
        file
    }

    #[test]
    fn parse_round_trips_to_json() {
        let file = sample();

        let parsed = VaultFile::parse(&file.to_json()).unwrap();

        assert_eq!(parsed.password_aad(), file.password_aad());
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
    fn parse_rejects_a_v3_file_without_its_data_key() {
        let mut file = sample();
        file.key = None;

        let parsed = VaultFile::parse(&file.to_json());

        assert!(matches!(parsed, Err(Error::Corrupt(_))));
    }

    #[test]
    fn password_aad_changes_when_salt_or_cost_changes() {
        let mut file = sample();
        let before = file.password_aad();
        file.salt = "b3RoZXI=".into();
        let after_salt = file.password_aad();
        file.kdf.t += 1;

        assert_ne!(after_salt, before);
        assert_ne!(file.password_aad(), after_salt);
    }

    #[test]
    fn password_and_recovery_aad_differ_for_the_same_salt() {
        let file = sample();

        assert_ne!(file.password_aad(), file.recovery_aad(&file.salt));
    }

    #[test]
    fn v3_file_writes_no_legacy_fields() {
        let raw = String::from_utf8(sample().to_json()).unwrap();

        assert!(!raw.contains("public_key"));
        assert!(!raw.contains("inbox"));
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
