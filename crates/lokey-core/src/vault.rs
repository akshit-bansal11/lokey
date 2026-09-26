//! Vault operations. Everything starts from `Store::unlock` (or `recover`, or
//! `restore`) and runs on the returned `Session`.
//!
//! The body is sealed under a random data key. The data key is sealed under
//! the master password's key and, when the owner chose one, under a recovery
//! key's key. Changing either one replaces the data key, so a leaked old
//! password or recovery key opens only copies of the file made before the
//! change.
//!
//! A `Session` holds the password's key and the data key in memory. Each
//! operation re-reads the file under the lock, so a change made meanwhile by
//! another process (an edit in a second window) is merged, not overwritten.

use std::{
    fs,
    path::{Path, PathBuf},
};

use zeroize::{Zeroize, Zeroizing};

pub use crate::format::Entry;
use crate::{
    Error, Result,
    crypto::{self, KdfParams, Key, SALT_LEN, b64, unb64},
    format::{Body, Record, Recovery, VERSION, VaultFile},
    lockout, names, password,
    store::{History, Store},
};

/// A recovery key as the owner writes it down: `ABCD-EFGH-…`. It is shown
/// once and never stored.
pub type RecoveryKey = Zeroizing<String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Replaced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub project: String,
    pub key: String,
    pub kind: ChangeKind,
}

/// What an unlock found in a v1/v2 vault's sealed inbox: the values merged in,
/// and the records that failed to open. Always empty for a v3 vault.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub changes: Vec<Change>,
    pub rejected: usize,
}

impl Store {
    /// Creates a new vault. With `recovery`, returns the recovery key that
    /// also opens it. Fails if a vault already exists.
    pub fn create(&self, master: &str, recovery: bool) -> Result<Option<RecoveryKey>> {
        password::check_new("master", master)?;
        let mut file = VaultFile::new(KdfParams::DEFAULT);
        let kek = new_password_key(&mut file, master)?;
        let (_, recovery_key) = rekey(&mut file, &kek, &Body::default(), recovery)?;

        self.locked(|store| {
            if store.exists() {
                return Err(Error::VaultExists);
            }
            store.write(&file, History::Keep)
        })?;
        Ok(recovery_key)
    }

    /// Opens the vault with the master password. A v1/v2 vault is upgraded to
    /// v3 here, merging its inbox.
    ///
    /// On a wrong password this returns `Error::WrongPassword` with a `wait`
    /// the caller must sleep before answering. The sleep is the caller's so
    /// that it never holds the file lock and blocks other processes.
    pub fn unlock(&self, master: &str) -> Result<(Session, Report)> {
        self.locked(|store| {
            let mut file = store.read()?;
            let (session, report, upgraded) = store.open_with(&mut file, master)?;
            if upgraded {
                store.write(&file, History::Keep)?;
            }
            Ok((session, report))
        })
    }

    /// Opens the vault with its recovery key and sets a new master password.
    /// Returns a new recovery key; the one used stops working.
    pub fn recover(&self, recovery_key: &str, new_master: &str) -> Result<(Session, RecoveryKey)> {
        password::check_new("master", new_master)?;
        let secret = crypto::parse_recovery_key(recovery_key).ok_or(Error::BadRecoveryKey)?;
        self.locked(|store| {
            lockout::check(store.dir())?;
            let mut file = store.read()?;
            let Some(recovery) = file.recovery.clone() else {
                return Err(Error::NoRecovery);
            };
            let rkek = crypto::derive_key(&secret[..], &unb64(&recovery.salt)?, file.kdf)?;
            let aad = file.recovery_aad(&recovery.salt);
            let body = match crypto::open_key(&rkek, &recovery.key, &aad)? {
                Some(dek) => open_body(&dek, &file)?,
                None => None,
            };
            let Some(body) = body else {
                return Err(lockout::fail(store.dir()));
            };
            lockout::clear(store.dir())?;

            let kek = new_password_key(&mut file, new_master)?;
            let (dek, _) = rekey(&mut file, &kek, &body, false)?;
            let fresh = seal_recovery(&mut file, &dek)?;
            store.write(&file, History::Forget)?;
            let session = Session {
                store: store.clone(),
                kek,
                dek,
                body,
            };
            Ok((session, fresh))
        })
    }

    /// Makes the backup at `backup` this PC's vault, if its master password
    /// opens it. Only when there is no vault here, so nothing is overwritten.
    pub fn restore(&self, backup: &Path, master: &str) -> Result<(Session, Report)> {
        let mut file = VaultFile::parse(&fs::read(backup)?)?;
        self.locked(|store| {
            if store.exists() {
                return Err(Error::VaultExists);
            }
            let (session, report, _) = store.open_with(&mut file, master)?;
            store.write(&file, History::Keep)?;
            Ok((session, report))
        })
    }

    /// Opens `file` with the master password; a wrong one counts toward the
    /// lockout. A v1/v2 file is upgraded to v3 in place, and `true` says the
    /// caller must write it.
    fn open_with(&self, file: &mut VaultFile, master: &str) -> Result<(Session, Report, bool)> {
        lockout::check(self.dir())?;
        let kek = crypto::derive_key(master.as_bytes(), &unb64(&file.salt)?, file.kdf)?;

        if file.is_legacy() {
            let Some(plain) = crypto::open(&kek, &file.body, &file.legacy_aad())? else {
                return Err(lockout::fail(self.dir()));
            };
            lockout::clear(self.dir())?;
            let mut body = parse_body(&plain)?;
            let report = merge_inbox(&mut body, file);
            let (dek, _) = rekey(file, &kek, &body, false)?;
            let session = Session {
                store: self.clone(),
                kek,
                dek,
                body,
            };
            return Ok((session, report, true));
        }

        let Some((dek, body)) = open_current(&kek, file)? else {
            return Err(lockout::fail(self.dir()));
        };
        lockout::clear(self.dir())?;
        let session = Session {
            store: self.clone(),
            kek,
            dek,
            body,
        };
        Ok((session, Report::default(), false))
    }
}

/// An unlocked vault. Dropping it zeroes both keys and every value.
pub struct Session {
    store: Store,
    /// The master password's key. It opens the sealed data key.
    kek: Key,
    dek: Key,
    body: Body,
}

impl Session {
    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn vault_path(&self) -> PathBuf {
        self.store.vault_path()
    }

    /// Every entry, in the order it was added.
    pub fn entries(&self) -> &[Entry] {
        &self.body.entries
    }

    /// Project names with their entry counts, `default` first, then by name.
    pub fn projects(&self) -> Vec<(String, usize)> {
        let mut projects: Vec<(String, usize)> = Vec::new();
        for entry in &self.body.entries {
            match projects
                .iter_mut()
                .find(|(name, _)| names::same(name, &entry.project))
            {
                Some((_, count)) => *count += 1,
                None => projects.push((entry.project.clone(), 1)),
            }
        }
        projects.sort_by_key(|(name, _)| {
            (
                !names::same(name, names::DEFAULT_PROJECT),
                name.to_lowercase(),
            )
        });
        projects
    }

    pub fn get(&self, project: &str, key: &str) -> Result<&Entry> {
        self.body
            .entries
            .iter()
            .find(|entry| names::same(&entry.project, project) && names::same(&entry.key, key))
            .ok_or_else(|| Error::NotFound {
                project: project.into(),
                key: key.into(),
            })
    }

    /// Re-reads the file, picking up changes made by another process.
    pub fn refresh(&mut self) -> Result<()> {
        let store = self.store.clone();
        store.locked(|store| self.reload(&store.read()?))
    }

    /// Adds or replaces a value.
    pub fn put(&mut self, project: &str, key: &str, value: &str) -> Result<ChangeKind> {
        names::check_name("project", project)?;
        names::check_name("key", key)?;
        names::check_value(value)?;
        let now = lockout::now();
        self.transact(History::Keep, |body| {
            Ok(upsert(&mut body.entries, project, key, value, now).kind)
        })
    }

    pub fn delete_key(&mut self, project: &str, key: &str) -> Result<()> {
        self.transact(History::Forget, |body| {
            let before = body.entries.len();
            body.entries.retain(|entry| {
                !(names::same(&entry.project, project) && names::same(&entry.key, key))
            });
            if body.entries.len() == before {
                return Err(Error::NotFound {
                    project: project.into(),
                    key: key.into(),
                });
            }
            Ok(())
        })
    }

    /// Removes a project and every key in it. Returns how many keys went.
    pub fn delete_project(&mut self, project: &str) -> Result<usize> {
        self.transact(History::Forget, |body| {
            let before = body.entries.len();
            body.entries
                .retain(|entry| !names::same(&entry.project, project));
            match before - body.entries.len() {
                0 => Err(Error::NotFound {
                    project: project.into(),
                    key: "*".into(),
                }),
                removed => Ok(removed),
            }
        })
    }

    /// Removes every key in every project. The vault and its password stay.
    pub fn truncate(&mut self) -> Result<usize> {
        self.transact(History::Forget, |body| {
            let removed = body.entries.len();
            body.entries.clear();
            Ok(removed)
        })
    }

    /// Changes the master password. Other sessions go `Stale`. When the vault
    /// has a recovery key, returns its replacement: the old one stops working,
    /// because the data key it opened is replaced.
    pub fn change_master(&mut self, new_master: &str) -> Result<Option<RecoveryKey>> {
        password::check_new("master", new_master)?;
        let store = self.store.clone();
        store.locked(|store| {
            let mut file = store.read()?;
            self.reload(&file)?;
            let recovery = file.recovery.is_some();
            let kek = new_password_key(&mut file, new_master)?;
            let (dek, recovery_key) = rekey(&mut file, &kek, &self.body, recovery)?;
            store.write(&file, History::Forget)?;
            self.kek = kek;
            self.dek = dek;
            Ok(recovery_key)
        })
    }

    /// Issues a new recovery key (`on`) or removes it. The data key is
    /// replaced either way, so a previous recovery key opens nothing saved
    /// from now on.
    pub fn set_recovery(&mut self, on: bool) -> Result<Option<RecoveryKey>> {
        let store = self.store.clone();
        store.locked(|store| {
            let mut file = store.read()?;
            self.reload(&file)?;
            let (dek, recovery_key) = rekey(&mut file, &self.kek, &self.body, on)?;
            store.write(&file, History::Forget)?;
            self.dek = dek;
            Ok(recovery_key)
        })
    }

    pub fn has_recovery(&self) -> bool {
        self.store.has_recovery()
    }

    /// Asks for the master password again, for actions that take a copy of
    /// the vault away. A wrong one counts toward the lockout like an unlock.
    pub fn verify_master(&self, master: &str) -> Result<()> {
        self.store.locked(|store| {
            lockout::check(store.dir())?;
            let file = store.read()?;
            let kek = crypto::derive_key(master.as_bytes(), &unb64(&file.salt)?, file.kdf)?;
            if open_data_key(&kek, &file)?.is_none() {
                return Err(lockout::fail(store.dir()));
            }
            lockout::clear(store.dir())
        })
    }

    /// Writes a backup to `dest`: the vault file as it is, still encrypted,
    /// opened later by the master password or the recovery key current now.
    pub fn export(&mut self, dest: &Path) -> Result<()> {
        let store = self.store.clone();
        store.locked(|store| {
            self.reload(&store.read()?)?;
            store.copy_to(dest)
        })
    }

    /// Lock, re-read, apply, write. `apply` sees the freshest body.
    fn transact<T>(
        &mut self,
        history: History,
        apply: impl FnOnce(&mut Body) -> Result<T>,
    ) -> Result<T> {
        let store = self.store.clone();
        store.locked(|store| {
            let mut file = store.read()?;
            self.reload(&file)?;
            let out = apply(&mut self.body)?;
            reseal(&self.dek, &self.body, &mut file)?;
            store.write(&file, history)?;
            Ok(out)
        })
    }

    /// Takes the data key and body from the file. `Stale` when the master
    /// password was changed elsewhere, since this session's key no longer
    /// opens it. A new recovery key made elsewhere is picked up silently.
    fn reload(&mut self, file: &VaultFile) -> Result<()> {
        if file.is_legacy() {
            return Err(Error::Stale);
        }
        let (dek, body) = open_current(&self.kek, file)?.ok_or(Error::Stale)?;
        self.dek = dek;
        self.body = body;
        Ok(())
    }
}

/// Derives the key for a new master password under a fresh salt, which is
/// recorded in `file`.
fn new_password_key(file: &mut VaultFile, master: &str) -> Result<Key> {
    let salt: [u8; SALT_LEN] = crypto::random();
    file.salt = b64(&salt);
    crypto::derive_key(master.as_bytes(), &salt, file.kdf)
}

/// Seals `body` into `file` as v3 under a fresh data key, sealed for `kek`
/// and, with `recovery`, for a fresh recovery key, which is returned.
fn rekey(
    file: &mut VaultFile,
    kek: &Key,
    body: &Body,
    recovery: bool,
) -> Result<(Key, Option<RecoveryKey>)> {
    file.version = VERSION;
    file.public_key.clear();
    file.inbox.clear();
    let dek = crypto::new_key();
    file.key = Some(crypto::seal(kek, &dek[..], &file.password_aad()));
    file.recovery = None;
    let recovery_key = recovery.then(|| seal_recovery(file, &dek)).transpose()?;
    reseal(&dek, body, file)?;
    Ok((dek, recovery_key))
}

/// Seals `dek` under a new recovery key and returns that key for the owner
/// to write down.
fn seal_recovery(file: &mut VaultFile, dek: &Key) -> Result<RecoveryKey> {
    let (secret, text) = crypto::new_recovery_key();
    let salt: [u8; SALT_LEN] = crypto::random();
    let rkek = crypto::derive_key(&secret[..], &salt, file.kdf)?;
    let salt = b64(&salt);
    let key = crypto::seal(&rkek, &dek[..], &file.recovery_aad(&salt));
    file.recovery = Some(Recovery { salt, key });
    Ok(text)
}

fn reseal(dek: &Key, body: &Body, file: &mut VaultFile) -> Result<()> {
    let plain = Zeroizing::new(
        serde_json::to_vec(body).map_err(|err| Error::Corrupt(format!("cannot save: {err}")))?,
    );
    file.body = crypto::seal(dek, &plain, &file.body_aad());
    Ok(())
}

/// The data key, if `kek` is the master password's key for this v3 file.
fn open_data_key(kek: &Key, file: &VaultFile) -> Result<Option<Key>> {
    let sealed = file
        .key
        .as_ref()
        .ok_or_else(|| Error::Corrupt("vault file has no data key".into()))?;
    crypto::open_key(kek, sealed, &file.password_aad())
}

/// The data key and body of a v3 file, if `kek` opens it.
fn open_current(kek: &Key, file: &VaultFile) -> Result<Option<(Key, Body)>> {
    let Some(dek) = open_data_key(kek, file)? else {
        return Ok(None);
    };
    Ok(open_body(&dek, file)?.map(|body| (dek, body)))
}

fn open_body(dek: &Key, file: &VaultFile) -> Result<Option<Body>> {
    crypto::open(dek, &file.body, &file.body_aad())?
        .map(|plain| parse_body(&plain))
        .transpose()
}

/// Moves the values in a v1/v2 inbox into the body, then forgets the key
/// that opened them. Only the removed command line wrote inbox records.
fn merge_inbox(body: &mut Body, file: &mut VaultFile) -> Report {
    let mut report = Report::default();
    let secret = Zeroizing::new(unb64(&body.inbox_secret).unwrap_or_default());
    body.inbox_secret.zeroize();
    for sealed in file.inbox.drain(..) {
        let record = unb64(&sealed)
            .ok()
            .and_then(|bytes| crypto::inbox_open(&secret, &bytes))
            .and_then(|plain| serde_json::from_slice::<Record>(&plain).ok())
            .filter(|record| {
                names::check_name("project", &record.project).is_ok()
                    && names::check_name("key", &record.key).is_ok()
                    && names::check_value(&record.value).is_ok()
            });
        match record {
            Some(record) => report.changes.push(upsert(
                &mut body.entries,
                &record.project,
                &record.key,
                &record.value,
                record.at,
            )),
            None => report.rejected += 1,
        }
    }
    report
}

fn upsert(entries: &mut Vec<Entry>, project: &str, key: &str, value: &str, at: u64) -> Change {
    let existing = entries
        .iter_mut()
        .find(|entry| names::same(&entry.project, project) && names::same(&entry.key, key));
    let kind = match existing {
        Some(entry) => {
            entry.value.zeroize();
            entry.value = value.into();
            entry.key = key.into();
            entry.updated = at;
            ChangeKind::Replaced
        }
        None => {
            entries.push(Entry {
                project: project.into(),
                key: key.into(),
                value: value.into(),
                updated: at,
            });
            ChangeKind::Added
        }
    };
    Change {
        project: project.into(),
        key: key.into(),
        kind,
    }
}

fn parse_body(plain: &[u8]) -> Result<Body> {
    serde_json::from_slice(plain)
        .map_err(|err| Error::Corrupt(format!("vault body is not valid: {err}")))
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    const MASTER: &str = "velvet otter plumbing ninety";
    const NEW_MASTER: &str = "brand new master phrase";

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let suffix: [u8; 8] = crypto::random();
            let name: String = suffix.iter().map(|b| format!("{b:02x}")).collect();
            let dir = env::temp_dir().join(format!("lokey-test-{name}"));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn store(&self) -> Store {
            Store::at(&self.0)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn new_vault() -> (TempDir, Store) {
        let dir = TempDir::new();
        let store = dir.store();
        store.create(MASTER, false).unwrap();
        (dir, store)
    }

    fn new_vault_with_recovery() -> (TempDir, Store, RecoveryKey) {
        let dir = TempDir::new();
        let store = dir.store();
        let recovery_key = store.create(MASTER, true).unwrap().unwrap();
        (dir, store, recovery_key)
    }

    fn keys(session: &Session) -> Vec<String> {
        session.entries().iter().map(|e| e.key.clone()).collect()
    }

    /// Writes a vault as 0.0.2 or earlier left it: the body sealed straight
    /// under the password's key, holding `OLD`, and an inbox sealed to the
    /// vault's public key.
    fn write_legacy(store: &Store, version: u32, deletion: bool, inbox: &[&[u8]]) {
        let pair = crypto::legacy::gen_keypair();
        let salt: [u8; SALT_LEN] = crypto::random();
        let mut file = VaultFile::new(KdfParams::DEFAULT);
        file.version = version;
        file.salt = b64(&salt);
        file.public_key = b64(&pair.public);
        let key = crypto::derive_key(MASTER.as_bytes(), &salt, file.kdf).unwrap();
        let mut body = serde_json::json!({
            "secret_key": b64(&pair.secret),
            "public_key": b64(&pair.public),
            "entries": [{ "project": "default", "key": "OLD", "value": "1", "updated": 0 }],
        });
        if deletion {
            body["deletion"] = serde_json::json!({ "salt": "c2FsdA==", "hash": "aGFzaA==" });
        }
        let plain = serde_json::to_vec(&body).unwrap();
        file.body = crypto::seal(&key, &plain, &file.legacy_aad());
        for record in inbox {
            let sealed = crypto::legacy::inbox_seal(&pair.public, record);
            file.inbox.push(b64(&sealed));
        }
        store.write(&file, History::Keep).unwrap();
    }

    fn inbox_record(key: &str, value: &str) -> Vec<u8> {
        let record = Record {
            project: "default".into(),
            key: key.into(),
            value: value.into(),
            at: 1,
        };
        serde_json::to_vec(&record).unwrap()
    }

    #[test]
    fn unlock_with_wrong_password_returns_wrong_password() {
        let (_dir, store) = new_vault();

        let result = store.unlock("wrong password entirely");

        assert!(matches!(
            result,
            Err(Error::WrongPassword {
                attempts_left: 9,
                ..
            })
        ));
    }

    #[test]
    fn ten_wrong_passwords_lock_the_vault() {
        let (_dir, store) = new_vault();
        let attempts: Vec<_> = (0..10)
            .map(|_| store.unlock("not it at all").err())
            .collect();

        let after = store.unlock(MASTER);

        assert!(matches!(attempts[9], Some(Error::LockedOut { .. })));
        assert!(matches!(after, Err(Error::LockedOut { .. })));
    }

    #[test]
    fn create_twice_returns_vault_exists() {
        let (_dir, store) = new_vault();

        let result = store.create(MASTER, false);

        assert!(matches!(result, Err(Error::VaultExists)));
    }

    #[test]
    fn put_is_visible_to_a_fresh_unlock() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();

        session.put("api", "DATABASE_URL", "postgres://x").unwrap();

        let (fresh, _) = store.unlock(MASTER).unwrap();
        assert_eq!(
            fresh.get("api", "database_url").unwrap().value,
            "postgres://x"
        );
    }

    #[test]
    fn put_writes_no_plaintext_to_disk() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();

        session.put("default", "A", "plain-marker-7731").unwrap();

        let raw = fs::read_to_string(store.vault_path()).unwrap();
        assert!(!raw.contains("plain-marker-7731"));
    }

    #[test]
    fn refresh_picks_up_a_put_from_another_session() {
        let (_dir, store) = new_vault();
        let (mut app, _) = store.unlock(MASTER).unwrap();
        let (mut other, _) = store.unlock(MASTER).unwrap();
        other.put("default", "FROM_OTHER", "1").unwrap();

        app.refresh().unwrap();

        assert_eq!(keys(&app), vec!["FROM_OTHER".to_string()]);
    }

    #[test]
    fn delete_key_removes_key_without_another_password() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("default", "GONE", "1").unwrap();

        session.delete_key("default", "gone").unwrap();

        assert!(session.entries().is_empty());
    }

    #[test]
    fn delete_key_also_removes_it_from_the_backup() {
        let (dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("default", "LEAKED", "1").unwrap();
        session.put("default", "OTHER", "2").unwrap();
        session.delete_key("default", "LEAKED").unwrap();
        let restored = TempDir::new();
        fs::copy(
            dir.0.join("vault.lokey.bak"),
            restored.0.join("vault.lokey"),
        )
        .unwrap();

        let (from_backup, _) = restored.store().unlock(MASTER).unwrap();

        assert_eq!(keys(&from_backup), vec!["OTHER".to_string()]);
    }

    #[test]
    fn delete_project_removes_only_that_project() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("web", "A", "1").unwrap();
        session.put("api", "B", "2").unwrap();

        let removed = session.delete_project("WEB").unwrap();

        assert_eq!(removed, 1);
        assert_eq!(keys(&session), vec!["B".to_string()]);
    }

    #[test]
    fn truncate_removes_every_entry() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("web", "A", "1").unwrap();
        session.put("api", "B", "2").unwrap();

        let removed = session.truncate().unwrap();

        assert_eq!(removed, 2);
        assert!(session.entries().is_empty());
    }

    #[test]
    fn change_master_password_makes_other_sessions_stale() {
        let (_dir, store) = new_vault();
        let (mut app, _) = store.unlock(MASTER).unwrap();
        let (mut other, _) = store.unlock(MASTER).unwrap();

        other.change_master(NEW_MASTER).unwrap();

        assert!(matches!(app.refresh(), Err(Error::Stale)));
    }

    #[test]
    fn change_master_password_old_password_stops_working() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.change_master(NEW_MASTER).unwrap();

        let old = store.unlock(MASTER);

        assert!(matches!(old, Err(Error::WrongPassword { .. })));
        assert!(store.unlock(NEW_MASTER).is_ok());
    }

    #[test]
    fn old_password_with_an_old_copy_cannot_read_what_is_saved_after_a_change() {
        let (_dir, store) = new_vault();
        let old_copy = store.read().unwrap();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.change_master(NEW_MASTER).unwrap();
        session.put("default", "AFTER", "secret").unwrap();
        let old_kek =
            crypto::derive_key(MASTER.as_bytes(), &unb64(&old_copy.salt).unwrap(), old_copy.kdf)
                .unwrap();
        let old_dek = open_data_key(&old_kek, &old_copy).unwrap().unwrap();

        let new_file = store.read().unwrap();

        assert!(open_body(&old_dek, &new_file).unwrap().is_none());
    }

    #[test]
    fn create_with_recovery_leaves_the_password_working() {
        let (_dir, store, recovery_key) = new_vault_with_recovery();

        let (session, _) = store.unlock(MASTER).unwrap();

        assert_eq!(recovery_key.len(), 39);
        assert!(session.has_recovery());
    }

    #[test]
    fn recover_sets_a_new_password_and_replaces_the_recovery_key() {
        let (_dir, store, first) = new_vault_with_recovery();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("default", "KEPT", "1").unwrap();

        let (recovered, second) = store.recover(&first, NEW_MASTER).unwrap();

        assert_eq!(keys(&recovered), vec!["KEPT".to_string()]);
        assert!(matches!(store.unlock(MASTER), Err(Error::WrongPassword { .. })));
        assert!(store.unlock(NEW_MASTER).is_ok());
        assert!(matches!(
            store.recover(&first, MASTER),
            Err(Error::WrongPassword { .. })
        ));
        assert!(store.recover(&second, MASTER).is_ok());
    }

    #[test]
    fn recover_with_another_key_counts_as_a_wrong_password() {
        let (_dir, store, _) = new_vault_with_recovery();
        let (_, stranger) = crypto::new_recovery_key();

        let result = store.recover(&stranger, NEW_MASTER);

        assert!(matches!(
            result,
            Err(Error::WrongPassword {
                attempts_left: 9,
                ..
            })
        ));
    }

    #[test]
    fn recover_with_a_mistyped_key_says_so_and_counts_nothing() {
        let (_dir, store, recovery_key) = new_vault_with_recovery();

        let result = store.recover(&recovery_key[2..], NEW_MASTER);

        assert!(matches!(result, Err(Error::BadRecoveryKey)));
        assert!(store.lockout_remaining().is_none());
        assert!(matches!(
            store.unlock("still not it"),
            Err(Error::WrongPassword {
                attempts_left: 9,
                ..
            })
        ));
    }

    #[test]
    fn recover_without_a_recovery_key_returns_no_recovery() {
        let (_dir, store) = new_vault();
        let (_, some_key) = crypto::new_recovery_key();

        let result = store.recover(&some_key, NEW_MASTER);

        assert!(matches!(result, Err(Error::NoRecovery)));
    }

    #[test]
    fn change_master_replaces_the_recovery_key() {
        let (_dir, store, first) = new_vault_with_recovery();
        let (mut session, _) = store.unlock(MASTER).unwrap();

        let second = session.change_master(NEW_MASTER).unwrap().unwrap();

        assert!(matches!(
            store.recover(&first, MASTER),
            Err(Error::WrongPassword { .. })
        ));
        assert!(store.recover(&second, MASTER).is_ok());
    }

    #[test]
    fn change_master_without_recovery_issues_none() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();

        let issued = session.change_master(NEW_MASTER).unwrap();

        assert!(issued.is_none());
        assert!(!store.has_recovery());
    }

    #[test]
    fn set_recovery_turns_it_on_and_off() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();

        let issued = session.set_recovery(true).unwrap().unwrap();
        let on = store.has_recovery();
        session.set_recovery(false).unwrap();

        assert!(on);
        assert!(!store.has_recovery());
        assert!(matches!(
            store.recover(&issued, NEW_MASTER),
            Err(Error::NoRecovery)
        ));
    }

    #[test]
    fn a_new_recovery_key_from_another_session_does_not_lock_this_one() {
        let (_dir, store, _) = new_vault_with_recovery();
        let (mut app, _) = store.unlock(MASTER).unwrap();
        let (mut other, _) = store.unlock(MASTER).unwrap();
        other.set_recovery(true).unwrap();

        app.put("default", "STILL_OPEN", "1").unwrap();

        assert_eq!(keys(&app), vec!["STILL_OPEN".to_string()]);
    }

    #[test]
    fn stripping_the_recovery_block_leaves_the_password_working() {
        let (_dir, store, _) = new_vault_with_recovery();
        let mut file = store.read().unwrap();
        file.recovery = None;
        store.write(&file, History::Keep).unwrap();

        assert!(store.unlock(MASTER).is_ok());
    }

    #[test]
    fn verify_master_with_a_wrong_password_counts_toward_the_lockout() {
        let (_dir, store) = new_vault();
        let (session, _) = store.unlock(MASTER).unwrap();

        let wrong = session.verify_master("wrong password entirely");

        assert!(matches!(
            wrong,
            Err(Error::WrongPassword {
                attempts_left: 9,
                ..
            })
        ));
        assert!(session.verify_master(MASTER).is_ok());
    }

    #[test]
    fn export_writes_a_backup_that_restores_on_another_pc() {
        let (dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("web", "TOKEN", "t-1").unwrap();
        let backup = dir.0.join("backup.lokey");

        session.export(&backup).unwrap();
        let elsewhere = TempDir::new();
        let (restored, _) = elsewhere.store().restore(&backup, MASTER).unwrap();

        assert_eq!(restored.get("web", "TOKEN").unwrap().value, "t-1");
        assert!(elsewhere.store().unlock(MASTER).is_ok());
    }

    #[test]
    fn restore_never_overwrites_an_existing_vault() {
        let (dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        let backup = dir.0.join("backup.lokey");
        session.export(&backup).unwrap();

        let result = store.restore(&backup, MASTER);

        assert!(matches!(result, Err(Error::VaultExists)));
    }

    #[test]
    fn restore_with_a_wrong_password_writes_nothing() {
        let (dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        let backup = dir.0.join("backup.lokey");
        session.export(&backup).unwrap();
        let elsewhere = TempDir::new();

        let result = elsewhere.store().restore(&backup, "wrong password entirely");

        assert!(matches!(result, Err(Error::WrongPassword { .. })));
        assert!(!elsewhere.store().exists());
    }

    #[test]
    fn v2_vault_with_an_inbox_is_merged_and_upgraded_to_v3() {
        let dir = TempDir::new();
        let store = dir.store();
        write_legacy(&store, 2, false, &[&inbox_record("NEW", "2")]);

        let (session, report) = store.unlock(MASTER).unwrap();

        assert_eq!(report.changes.len(), 1);
        assert_eq!(keys(&session), vec!["OLD".to_string(), "NEW".to_string()]);
        let raw = fs::read_to_string(store.vault_path()).unwrap();
        assert!(!raw.contains("inbox") && !raw.contains("public_key"));
        assert_eq!(store.read().unwrap().version, VERSION);
        assert_eq!(keys(&store.unlock(MASTER).unwrap().0).len(), 2);
    }

    #[test]
    fn v1_vault_with_a_deletion_check_opens_and_is_upgraded() {
        let dir = TempDir::new();
        let store = dir.store();
        write_legacy(&store, 1, true, &[]);

        let (session, _) = store.unlock(MASTER).unwrap();

        assert_eq!(keys(&session), vec!["OLD".to_string()]);
        let raw = fs::read_to_string(store.vault_path()).unwrap();
        assert!(!raw.contains("deletion"));
        assert_eq!(store.read().unwrap().version, VERSION);
    }

    #[test]
    fn garbage_inbox_record_is_rejected_not_merged() {
        let dir = TempDir::new();
        let store = dir.store();
        write_legacy(&store, 2, false, &[]);
        let mut file = store.read().unwrap();
        file.inbox
            .push(b64(b"not a sealed record at all, just junk bytes"));
        store.write(&file, History::Keep).unwrap();

        let (session, report) = store.unlock(MASTER).unwrap();

        assert_eq!(report.rejected, 1);
        assert_eq!(keys(&session), vec!["OLD".to_string()]);
    }

    #[test]
    fn modified_body_reads_as_wrong_password() {
        let (_dir, store) = new_vault();
        let mut file = store.read().unwrap();
        let mut ct = unb64(&file.body.ct).unwrap();
        ct[0] ^= 1;
        file.body.ct = b64(&ct);
        store.write(&file, History::Keep).unwrap();

        let result = store.unlock(MASTER);

        assert!(matches!(result, Err(Error::WrongPassword { .. })));
    }

    #[test]
    fn raised_cost_in_the_header_reads_as_wrong_password() {
        let (_dir, store) = new_vault();
        let mut file = store.read().unwrap();
        file.kdf.t += 1;
        store.write(&file, History::Keep).unwrap();

        let result = store.unlock(MASTER);

        assert!(matches!(result, Err(Error::WrongPassword { .. })));
    }

    #[test]
    fn projects_lists_default_first_with_counts() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("web", "A", "1").unwrap();
        session.put("default", "B", "2").unwrap();
        session.put("web", "C", "3").unwrap();

        let projects = session.projects();

        assert_eq!(
            projects,
            vec![("default".to_string(), 1), ("web".to_string(), 2)]
        );
    }
}
