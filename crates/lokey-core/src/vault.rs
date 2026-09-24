//! Vault operations. `Store::set` seals a value into the inbox and needs no
//! password; everything else starts from `Store::unlock` and runs on the
//! returned `Session`. Nothing shipping calls `set` since the command line was
//! removed, but a vault written by one can still hold inbox records, so the
//! merge on unlock stays.
//!
//! A `Session` holds the derived master key in memory. Each operation re-reads
//! the file under the lock, so a change made meanwhile by another process (an
//! edit in a second window) is merged rather than overwritten.

use std::path::PathBuf;

use zeroize::{Zeroize, Zeroizing};

pub use crate::format::Entry;
use crate::{
    Error, Result,
    crypto::{self, KEY_LEN, KdfParams, SALT_LEN, b64, unb64},
    format::{Body, Record, VERSION, VaultFile},
    lockout, names, password,
    store::{History, Store},
};

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

/// What happened to the vault since this process last looked: values that
/// arrived through password-less `set`, records that failed to open, and
/// whether the header's public key had been swapped and was put back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub changes: Vec<Change>,
    pub rejected: usize,
    pub header_restored: bool,
}

impl Report {
    fn touched(&self) -> bool {
        !self.changes.is_empty() || self.rejected > 0 || self.header_restored
    }
}

impl Store {
    /// Creates a new vault. Fails if one already exists.
    pub fn create(&self, master: &str) -> Result<()> {
        password::check_new("master", master)?;

        let kdf = KdfParams::DEFAULT;
        let salt: [u8; SALT_LEN] = crypto::random();
        let key = crypto::derive_key(master.as_bytes(), &salt, kdf)?;
        let pair = crypto::gen_keypair();
        let public_key = b64(&pair.public);
        let body = Body {
            secret_key: b64(&pair.secret),
            public_key: public_key.clone(),
            entries: Vec::new(),
        };

        let placeholder = crypto::Sealed {
            nonce: String::new(),
            ct: String::new(),
        };
        let mut file = VaultFile::new(kdf, b64(&salt), public_key, placeholder);
        reseal(&key, &body, &mut file)?;

        self.locked(|store| {
            if store.exists() {
                return Err(Error::VaultExists);
            }
            store.write(&file, History::Keep)
        })
    }

    /// Adds or replaces a value without any password. The value is sealed to
    /// the vault's public key; only the master password can read it back.
    pub fn set(&self, project: &str, key: &str, value: &str) -> Result<()> {
        self.set_many(project, &[(key, value)])
    }

    /// `set` for many pairs in one locked write. Every name and value is
    /// checked first, so either all pairs land or none do.
    pub fn set_many(&self, project: &str, pairs: &[(&str, &str)]) -> Result<()> {
        names::check_name("project", project)?;
        for (key, value) in pairs {
            names::check_name("key", key)?;
            names::check_value(value)?;
        }
        let at = lockout::now();
        let plains: Vec<Zeroizing<Vec<u8>>> = pairs
            .iter()
            .map(|(key, value)| {
                let record = Record {
                    project: project.into(),
                    key: (*key).into(),
                    value: (*value).into(),
                    at,
                };
                Zeroizing::new(serde_json::to_vec(&record).expect("record serialises"))
            })
            .collect();

        self.locked(|store| {
            let mut file = store.read()?;
            let public_key = unb64(&file.public_key)?;
            for plain in &plains {
                file.inbox
                    .push(b64(&crypto::inbox_seal(&public_key, plain)?));
            }
            store.write(&file, History::Keep)
        })
    }

    /// Opens the vault with the master password and merges pending `set`s.
    ///
    /// On a wrong password this returns `Error::WrongPassword` with a `wait`
    /// the caller must sleep before answering. The sleep is the caller's so
    /// that it never holds the file lock and blocks other processes.
    pub fn unlock(&self, master: &str) -> Result<(Session, Report)> {
        self.locked(|store| {
            lockout::check(store.dir())?;
            let file = store.read()?;
            let key = crypto::derive_key(master.as_bytes(), &unb64(&file.salt)?, file.kdf)?;
            let Some(plain) = crypto::open(&key, &file.body, &file.aad())? else {
                return Err(lockout::fail(store.dir()));
            };
            lockout::clear(store.dir())?;

            let mut session = Session {
                store: store.clone(),
                key,
                salt: file.salt.clone(),
                kdf: file.kdf,
                body: parse_body(&plain)?,
            };
            let mut file = file;
            let report = session.merge_inbox(&mut file);
            if report.touched() {
                reseal(&session.key, &session.body, &mut file)?;
                store.write(&file, History::Keep)?;
            }
            Ok((session, report))
        })
    }
}

/// An unlocked vault. Dropping it zeroes the key and every value.
pub struct Session {
    store: Store,
    key: Zeroizing<[u8; KEY_LEN]>,
    salt: String,
    kdf: KdfParams,
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

    /// Re-reads the file and merges anything added since. Writes only when
    /// something changed, so polling this is cheap.
    pub fn refresh(&mut self) -> Result<Report> {
        let store = self.store.clone();
        store.locked(|store| {
            let mut file = store.read()?;
            self.reload(&file)?;
            let report = self.merge_inbox(&mut file);
            if report.touched() {
                reseal(&self.key, &self.body, &mut file)?;
                store.write(&file, History::Keep)?;
            }
            Ok(report)
        })
    }

    /// Adds or replaces a value with the vault already unlocked.
    pub fn put(&mut self, project: &str, key: &str, value: &str) -> Result<(ChangeKind, Report)> {
        names::check_name("project", project)?;
        names::check_name("key", key)?;
        names::check_value(value)?;
        let now = lockout::now();
        self.transact(History::Keep, |body| {
            Ok(upsert(&mut body.entries, project, key, value, now).kind)
        })
    }

    pub fn delete_key(&mut self, project: &str, key: &str) -> Result<Report> {
        let (_, report) = self.transact(History::Forget, |body| {
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
        })?;
        Ok(report)
    }

    /// Removes a project and every key in it. Returns how many keys went.
    pub fn delete_project(&mut self, project: &str) -> Result<(usize, Report)> {
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
    pub fn truncate(&mut self) -> Result<(usize, Report)> {
        self.transact(History::Forget, |body| {
            let removed = body.entries.len();
            body.entries.clear();
            Ok(removed)
        })
    }

    /// Changes the master password. Other sessions go `Stale`.
    pub fn change_master(&mut self, new_master: &str) -> Result<()> {
        password::check_new("master", new_master)?;
        let store = self.store.clone();
        store.locked(|store| {
            let mut file = store.read()?;
            self.reload(&file)?;
            self.merge_inbox(&mut file);
            let salt: [u8; SALT_LEN] = crypto::random();
            self.key = crypto::derive_key(new_master.as_bytes(), &salt, self.kdf)?;
            self.salt = b64(&salt);
            file.salt = self.salt.clone();
            reseal(&self.key, &self.body, &mut file)?;
            store.write(&file, History::Forget)
        })
    }

    /// Lock, re-read, merge, apply, write. `apply` sees the freshest body.
    fn transact<T>(
        &mut self,
        history: History,
        apply: impl FnOnce(&mut Body) -> Result<T>,
    ) -> Result<(T, Report)> {
        let store = self.store.clone();
        store.locked(|store| {
            let mut file = store.read()?;
            self.reload(&file)?;
            let report = self.merge_inbox(&mut file);
            let out = apply(&mut self.body)?;
            reseal(&self.key, &self.body, &mut file)?;
            store.write(&file, history)?;
            Ok((out, report))
        })
    }

    /// Replaces the in-memory body with the file's. `Stale` when the file was
    /// re-keyed elsewhere, since this session's key no longer opens it.
    fn reload(&mut self, file: &VaultFile) -> Result<()> {
        if file.salt != self.salt || file.kdf != self.kdf {
            return Err(Error::Stale);
        }
        let plain = crypto::open(&self.key, &file.body, &file.aad())?.ok_or(Error::Stale)?;
        self.body = parse_body(&plain)?;
        Ok(())
    }

    fn merge_inbox(&mut self, file: &mut VaultFile) -> Report {
        let mut report = Report::default();
        if file.public_key != self.body.public_key {
            file.public_key = self.body.public_key.clone();
            report.header_restored = true;
        }
        let secret = Zeroizing::new(unb64(&self.body.secret_key).unwrap_or_default());
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
                    &mut self.body.entries,
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

/// Seals `body` into `file` as the current format version. The version is in
/// the associated data, so it is set before sealing.
fn reseal(key: &[u8; KEY_LEN], body: &Body, file: &mut VaultFile) -> Result<()> {
    let plain = Zeroizing::new(
        serde_json::to_vec(body).map_err(|err| Error::Corrupt(format!("cannot save: {err}")))?,
    );
    file.version = VERSION;
    file.body = crypto::seal(key, &plain, &file.aad());
    Ok(())
}

fn parse_body(plain: &[u8]) -> Result<Body> {
    serde_json::from_slice(plain)
        .map_err(|err| Error::Corrupt(format!("vault body is not valid: {err}")))
}

#[cfg(test)]
mod tests {
    use std::{env, fs, thread};

    use super::*;

    const MASTER: &str = "velvet otter plumbing ninety";

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
        store.create(MASTER).unwrap();
        (dir, store)
    }

    fn keys(session: &Session) -> Vec<String> {
        session.entries().iter().map(|e| e.key.clone()).collect()
    }

    fn set_many_in_parallel(store: &Store, threads: usize, per_thread: usize) {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let store = store.clone();
                thread::spawn(move || {
                    for i in 0..per_thread {
                        store.set("default", &format!("K_{t}_{i}"), "v").unwrap();
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn set_then_unlock_returns_value() {
        let (_dir, store) = new_vault();
        store.set("web", "API_KEY", "sk-123").unwrap();

        let (session, _) = store.unlock(MASTER).unwrap();

        assert_eq!(session.get("web", "api_key").unwrap().value, "sk-123");
    }

    #[test]
    fn set_same_key_twice_keeps_newest_and_reports_replaced() {
        let (_dir, store) = new_vault();
        store.set("default", "TOKEN", "old").unwrap();
        store.set("default", "TOKEN", "new").unwrap();

        let (session, report) = store.unlock(MASTER).unwrap();

        assert_eq!(session.get("default", "TOKEN").unwrap().value, "new");
        assert_eq!(report.changes[1].kind, ChangeKind::Replaced);
    }

    #[test]
    fn set_many_with_one_bad_name_saves_nothing() {
        let (_dir, store) = new_vault();

        let result = store.set_many("default", &[("GOOD", "1"), ("BAD=NAME", "2")]);

        assert!(matches!(result, Err(Error::InvalidName(_))));
        assert_eq!(store.pending().unwrap(), 0);
    }

    #[test]
    fn set_without_vault_returns_no_vault() {
        let dir = TempDir::new();

        let result = dir.store().set("default", "A", "1");

        assert!(matches!(result, Err(Error::NoVault)));
    }

    #[test]
    fn set_writes_no_plaintext_to_disk() {
        let (_dir, store) = new_vault();

        store.set("default", "A", "plain-marker-7731").unwrap();

        let raw = fs::read_to_string(store.vault_path()).unwrap();
        assert!(!raw.contains("plain-marker-7731"));
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

        let result = store.create(MASTER);

        assert!(matches!(result, Err(Error::VaultExists)));
    }

    #[test]
    fn put_is_visible_to_a_fresh_unlock() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();

        session.put("api", "DATABASE_URL", "postgres://x").unwrap();

        let (fresh, _) = store.unlock(MASTER).unwrap();
        assert_eq!(
            fresh.get("api", "DATABASE_URL").unwrap().value,
            "postgres://x"
        );
    }

    #[test]
    fn refresh_picks_up_set_from_another_writer() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        store.set("default", "FROM_CLI", "1").unwrap();

        let report = session.refresh().unwrap();

        assert_eq!(report.changes[0].key, "FROM_CLI");
        assert_eq!(keys(&session), vec!["FROM_CLI".to_string()]);
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

        let (removed, _) = session.delete_project("WEB").unwrap();

        assert_eq!(removed, 1);
        assert_eq!(keys(&session), vec!["B".to_string()]);
    }

    #[test]
    fn truncate_removes_every_entry() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("web", "A", "1").unwrap();
        session.put("api", "B", "2").unwrap();

        let (removed, _) = session.truncate().unwrap();

        assert_eq!(removed, 2);
        assert!(session.entries().is_empty());
    }

    #[test]
    fn change_master_password_makes_other_sessions_stale() {
        let (_dir, store) = new_vault();
        let (mut app, _) = store.unlock(MASTER).unwrap();
        let (mut cli, _) = store.unlock(MASTER).unwrap();

        cli.change_master("brand new master phrase").unwrap();

        assert!(matches!(app.refresh(), Err(Error::Stale)));
    }

    #[test]
    fn change_master_password_old_password_stops_working() {
        let (_dir, store) = new_vault();
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.change_master("brand new master phrase").unwrap();

        let old = store.unlock(MASTER);

        assert!(matches!(old, Err(Error::WrongPassword { .. })));
        assert!(store.unlock("brand new master phrase").is_ok());
    }

    #[test]
    fn v1_vault_with_a_deletion_check_opens_and_is_upgraded() {
        let (_dir, store) = new_vault();
        let mut file = store.read().unwrap();
        // Rewrite the vault as 1.0.0 left it: version 1, a deletion check in the body.
        let salt = unb64(&file.salt).unwrap();
        let key = crypto::derive_key(MASTER.as_bytes(), &salt, file.kdf).unwrap();
        let plain = crypto::open(&key, &file.body, &file.aad())
            .unwrap()
            .unwrap();
        let mut body: serde_json::Value = serde_json::from_slice(&plain).unwrap();
        body["deletion"] = serde_json::json!({ "salt": "c2FsdA==", "hash": "aGFzaA==" });
        file.version = 1;
        file.body = crypto::seal(&key, &serde_json::to_vec(&body).unwrap(), &file.aad());
        store.write(&file, History::Keep).unwrap();

        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("default", "A", "1").unwrap();

        assert_eq!(store.read().unwrap().version, VERSION);
        let raw = fs::read_to_string(store.vault_path()).unwrap();
        assert!(!raw.contains("deletion"));
        assert_eq!(
            keys(&store.unlock(MASTER).unwrap().0),
            vec!["A".to_string()]
        );
    }

    #[test]
    fn swapped_header_public_key_is_restored_and_reported() {
        let (_dir, store) = new_vault();
        let mut file = store.read().unwrap();
        file.public_key = b64(&crypto::gen_keypair().public);
        store.write(&file, History::Keep).unwrap();

        let (_, report) = store.unlock(MASTER).unwrap();

        assert!(report.header_restored);
        assert_ne!(store.read().unwrap().public_key, file.public_key);
    }

    #[test]
    fn garbage_inbox_record_is_rejected_not_merged() {
        let (_dir, store) = new_vault();
        let mut file = store.read().unwrap();
        file.inbox
            .push(b64(b"not a sealed record at all, just junk bytes"));
        store.write(&file, History::Keep).unwrap();

        let (session, report) = store.unlock(MASTER).unwrap();

        assert_eq!(report.rejected, 1);
        assert!(session.entries().is_empty());
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
    fn parallel_sets_from_many_writers_all_arrive() {
        let (_dir, store) = new_vault();
        set_many_in_parallel(&store, 8, 5);

        let (session, _) = store.unlock(MASTER).unwrap();

        assert_eq!(session.entries().len(), 40);
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
