//! Every cryptographic primitive lokey uses, in one file.
//!
//! - AES-256-GCM seals the vault body under a key derived from the master password.
//! - Argon2id derives that key.
//! - HPKE (RFC 9180, DHKEM-X25519 / HKDF-SHA256 / AES-256-GCM) seals `set`
//!   records to the vault's public key, which is how a value can be added
//!   without the master password while only the master password can read it.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine, engine::general_purpose::STANDARD};
use hpke::{
    Deserializable, Kem as _, OpModeR, OpModeS, Serializable, aead::AesGcm256, kdf::HkdfSha256,
    kem::X25519HkdfSha256,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{Error, Result};

pub const KEY_LEN: usize = 32;
pub const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
/// An X25519 encapsulated key is 32 bytes; it prefixes every inbox record.
const ENCAPPED_LEN: usize = 32;

type Kem = X25519HkdfSha256;
const HPKE_INFO: &[u8] = b"lokey inbox v1";

/// Argon2id cost, stored in the vault header so it can be raised later
/// without breaking existing vaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl KdfParams {
    /// SECURITY-SPEC SEC-05: Argon2id at 64 MiB, t=3, p=1, above the OWASP
    /// floor of 19 MiB, t=2, p=1. Never lowered, not even for tests.
    pub const DEFAULT: Self = Self {
        m_kib: 64 * 1024,
        t: 3,
        p: 1,
    };

    /// Refuse headers asking for less than the OWASP floor (a downgraded
    /// file) or more than 1 GiB (a file crafted to exhaust memory on open).
    pub fn check(&self) -> Result<()> {
        let sane = (19 * 1024..=1024 * 1024).contains(&self.m_kib)
            && (1..=16).contains(&self.t)
            && (1..=8).contains(&self.p);
        if sane {
            Ok(())
        } else {
            Err(Error::Corrupt(
                "vault header has unsafe key-derivation settings".into(),
            ))
        }
    }
}

pub fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    // An OS RNG failure leaves nothing safe to fall back to.
    getrandom::fill(&mut bytes).expect("the operating system's random number generator failed");
    bytes
}

pub fn derive_key(
    password: &[u8],
    salt: &[u8],
    params: KdfParams,
) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    params.check()?;
    let argon_params = Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN))
        .map_err(|err| Error::Corrupt(format!("key-derivation settings rejected: {err}")))?;
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params)
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|err| Error::Corrupt(format!("key derivation failed: {err}")))?;
    Ok(key)
}

/// AES-256-GCM output: a fresh random nonce and the ciphertext with its tag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    pub nonce: String,
    pub ct: String,
}

pub fn seal(key: &[u8; KEY_LEN], plaintext: &[u8], aad: &[u8]) -> Sealed {
    let cipher = Aes256Gcm::new(&(*key).into());
    let nonce_bytes: [u8; NONCE_LEN] = random();
    let ct = cipher
        .encrypt(
            &Nonce::from(nonce_bytes),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        // Only fails on inputs of 64 GiB and up, which the size caps rule out.
        .expect("AES-GCM input within size limits");
    Sealed {
        nonce: b64(&nonce_bytes),
        ct: b64(&ct),
    }
}

/// `None` means authentication failed: a wrong key or a modified file. GCM
/// cannot tell those apart, and callers treat both as "wrong password".
pub fn open(
    key: &[u8; KEY_LEN],
    sealed: &Sealed,
    aad: &[u8],
) -> Result<Option<Zeroizing<Vec<u8>>>> {
    let nonce_bytes: [u8; NONCE_LEN] = unb64(&sealed.nonce)?
        .try_into()
        .map_err(|_| Error::Corrupt("vault nonce has the wrong length".into()))?;
    let ct = unb64(&sealed.ct)?;
    let cipher = Aes256Gcm::new(&(*key).into());
    Ok(cipher
        .decrypt(&Nonce::from(nonce_bytes), Payload { msg: &ct, aad })
        .ok()
        .map(Zeroizing::new))
}

pub struct KeyPair {
    pub secret: Zeroizing<Vec<u8>>,
    pub public: Vec<u8>,
}

pub fn gen_keypair() -> KeyPair {
    let (sk, pk) = Kem::gen_keypair();
    KeyPair {
        secret: Zeroizing::new(sk.to_bytes().to_vec()),
        public: pk.to_bytes().to_vec(),
    }
}

/// Seals one inbox record to the vault's public key: `encapped_key || ciphertext`.
pub fn inbox_seal(public_key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let pk = <Kem as hpke::Kem>::PublicKey::from_bytes(public_key)
        .map_err(|_| Error::Corrupt("vault public key is malformed".into()))?;
    let (encapped, ct) = hpke::single_shot_seal::<AesGcm256, HkdfSha256, Kem>(
        &OpModeS::Base,
        &pk,
        HPKE_INFO,
        plaintext,
        b"",
    )
    .map_err(|err| Error::Corrupt(format!("could not seal the value: {err}")))?;
    let mut record = encapped.to_bytes().to_vec();
    record.extend_from_slice(&ct);
    Ok(record)
}

/// `None` means the record was not sealed to this key or was modified.
pub fn inbox_open(secret_key: &[u8], record: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if record.len() <= ENCAPPED_LEN {
        return None;
    }
    let (encapped, ct) = record.split_at(ENCAPPED_LEN);
    let sk = <Kem as hpke::Kem>::PrivateKey::from_bytes(secret_key).ok()?;
    let encapped = <Kem as hpke::Kem>::EncappedKey::from_bytes(encapped).ok()?;
    hpke::single_shot_open::<AesGcm256, HkdfSha256, Kem>(
        &OpModeR::Base,
        &sk,
        &encapped,
        HPKE_INFO,
        ct,
        b"",
    )
    .ok()
    .map(Zeroizing::new)
}

pub fn b64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn unb64(text: &str) -> Result<Vec<u8>> {
    STANDARD
        .decode(text)
        .map_err(|_| Error::Corrupt("vault contains invalid base64".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AAD: &[u8] = b"header";

    #[test]
    fn seal_then_open_with_same_key_returns_plaintext() {
        let key = [7u8; KEY_LEN];

        let sealed = seal(&key, b"postgres://db", AAD);

        assert_eq!(
            open(&key, &sealed, AAD).unwrap().unwrap().as_slice(),
            b"postgres://db"
        );
    }

    #[test]
    fn open_with_other_key_fails_authentication() {
        let sealed = seal(&[7u8; KEY_LEN], b"secret", AAD);

        let opened = open(&[8u8; KEY_LEN], &sealed, AAD).unwrap();

        assert!(opened.is_none());
    }

    #[test]
    fn open_with_changed_aad_fails_authentication() {
        let key = [7u8; KEY_LEN];
        let sealed = seal(&key, b"secret", AAD);

        let opened = open(&key, &sealed, b"downgraded header").unwrap();

        assert!(opened.is_none());
    }

    #[test]
    fn seal_twice_uses_fresh_nonces() {
        let key = [7u8; KEY_LEN];

        let first = seal(&key, b"same", AAD);
        let second = seal(&key, b"same", AAD);

        assert_ne!(first.nonce, second.nonce);
    }

    #[test]
    fn inbox_record_opens_only_with_matching_secret_key() {
        let pair = gen_keypair();
        let other = gen_keypair();
        let record = inbox_seal(&pair.public, b"API_KEY=1").unwrap();

        let opened = inbox_open(&pair.secret, &record);
        let wrong = inbox_open(&other.secret, &record);

        assert_eq!(opened.unwrap().as_slice(), b"API_KEY=1");
        assert!(wrong.is_none());
    }

    #[test]
    fn inbox_record_with_flipped_bit_is_rejected() {
        let pair = gen_keypair();
        let mut record = inbox_seal(&pair.public, b"value").unwrap();
        let last = record.len() - 1;
        record[last] ^= 1;

        assert!(inbox_open(&pair.secret, &record).is_none());
    }

    #[test]
    fn derive_key_is_deterministic_for_same_inputs() {
        let salt = [1u8; SALT_LEN];

        let first = derive_key(b"correct horse battery", &salt, KdfParams::DEFAULT).unwrap();
        let second = derive_key(b"correct horse battery", &salt, KdfParams::DEFAULT).unwrap();

        assert_eq!(*first, *second);
    }

    #[test]
    fn kdf_params_below_owasp_floor_are_refused() {
        let weak = KdfParams {
            m_kib: 1024,
            t: 1,
            p: 1,
        };

        assert!(matches!(weak.check(), Err(Error::Corrupt(_))));
    }
}
