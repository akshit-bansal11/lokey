//! Where the vault lives and how it is read and written.
//!
//! Two app windows are separate processes sharing one file. Every change
//! is "take the lock, re-read, change, write, release", so two writers can
//! never overwrite each other's change. Writes go to a temporary file that is
//! renamed over the vault, so a crash never leaves a half-written vault.

use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use crate::{Error, Result, crypto, format::VaultFile, lockout};

// The file names keep the extension from when the app was called lokey: the
// vault format, its name and its extension are unchanged by the rename.
const VAULT: &str = "vault.lokey";
const BACKUP: &str = "vault.lokey.bak";
const LOCK: &str = "vault.lokey.lock";

/// The vault folder's name, under `%LOCALAPPDATA%` on Windows.
const DIR: &str = "saltcellar";
/// The folder's name under the app's previous name. A vault found there is
/// copied over once; see [`Migration`].
const OLD_DIR: &str = "lokey";
/// Chooses the vault folder, overriding the default.
const DIR_VAR: &str = "SALTCELLAR_DIR";
/// The same, under the app's previous name. Still honoured.
const OLD_DIR_VAR: &str = "LOKEY_DIR";
/// What is copied from the old folder, the vault last: a `vault.lokey` in the
/// new folder means the copy finished. The lock file is not state.
const CARRIED: [&str; 3] = [lockout::FILE, BACKUP, VAULT];
/// One line per vault copied, or per failed attempt, beside the vault.
const MIGRATION_LOG: &str = "migration.log";

/// What became of a vault left in the folder of the app's previous name,
/// `%LOCALAPPDATA%\lokey`. The old folder is only ever read: it is never
/// written to, moved or deleted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Migration {
    /// The vault was copied from this folder.
    Copied { from: PathBuf },
    /// This folder already holds a vault, so the old one was left alone.
    NewVaultExists,
    /// There is no old vault to copy.
    NoOldVault,
    /// `SALTCELLAR_DIR` or `LOKEY_DIR` chose the folder, so nothing is copied.
    FolderOverride,
    /// Copying failed. No `vault.lokey` was written here, so the next start
    /// tries again.
    Failed { from: PathBuf, error: String },
}

/// Where the vault folder is, before anything touches it.
#[derive(Debug, PartialEq, Eq)]
enum Location {
    /// Named by an environment variable.
    Chosen(PathBuf),
    /// The default folder, and the one the app used under its previous name.
    Default { dir: PathBuf, old: PathBuf },
}

/// `SALTCELLAR_DIR`, then `LOKEY_DIR`, then the default under
/// `%LOCALAPPDATA%` (or the XDG data folder, for tests off Windows). `var`
/// reads one environment variable, so tests need not change the real ones.
fn locate(var: impl Fn(&'static str) -> Option<OsString>) -> Option<Location> {
    if let Some(dir) = var(DIR_VAR).or_else(|| var(OLD_DIR_VAR)) {
        return Some(Location::Chosen(PathBuf::from(dir)));
    }
    let base = var("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| var("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
    Some(Location::Default {
        dir: base.join(DIR),
        old: base.join(OLD_DIR),
    })
}

/// Whether the previous version may survive in `vault.lokey.bak`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum History {
    Keep,
    /// After a deletion or a password change the backup is overwritten too,
    /// so a deleted value or an old password does not live on in it.
    Forget,
}

#[derive(Clone, Debug)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// `SALTCELLAR_DIR` if set, then `LOKEY_DIR`, otherwise
    /// `%LOCALAPPDATA%\saltcellar` on Windows. Local, not Roaming, so Windows
    /// never syncs the vault to another machine.
    ///
    /// In the default folder, a vault left in `%LOCALAPPDATA%\lokey` by the
    /// app's previous name is copied over first, once. Call this before
    /// anything else reads or writes the vault.
    pub fn open_default() -> Result<(Self, Migration)> {
        match locate(env::var_os) {
            Some(Location::Chosen(dir)) => Ok((Self::at(dir), Migration::FolderOverride)),
            Some(Location::Default { dir, old }) => {
                let store = Self::at(dir);
                let migration = store.adopt(&old);
                Ok((store, migration))
            }
            None => Err(Error::Unsupported(
                "cannot find a data folder; set SALTCELLAR_DIR to choose one",
            )),
        }
    }

    /// Copies the vault in `old` into this folder, unless this folder already
    /// has one. Copies, never moves: `old` is only read. The vault is copied
    /// last and renamed into place, so a copy cut short leaves no
    /// `vault.lokey` here and the next start tries again.
    pub(crate) fn adopt(&self, old: &Path) -> Migration {
        if !old.join(VAULT).is_file() {
            return Migration::NoOldVault;
        }
        // This folder's lock: two windows starting at once copy only once.
        let outcome = self.locked(|store| {
            if store.exists() {
                return Ok(Migration::NewVaultExists);
            }
            copy_from(old, &store.dir)?;
            Ok(Migration::Copied {
                from: old.to_path_buf(),
            })
        });
        let migration = outcome.unwrap_or_else(|err| Migration::Failed {
            from: old.to_path_buf(),
            error: err.to_string(),
        });
        self.log(&migration);
        migration
    }

    /// Records a copy or a failed one in `migration.log`. The app has no
    /// console, so this file is where a migration can be checked afterwards.
    fn log(&self, migration: &Migration) {
        let line = match migration {
            Migration::Copied { from } => {
                format!("migrated: copied the vault from {}", from.display())
            }
            Migration::Failed { from, error } => {
                format!(
                    "failed: could not copy the vault from {}: {error}",
                    from.display()
                )
            }
            Migration::NewVaultExists | Migration::NoOldVault | Migration::FolderOverride => {
                return;
            }
        };
        // Best effort: a log that cannot be written must not stop the app.
        let _ = append_line(
            &self.dir.join(MIGRATION_LOG),
            &format!("{} {line}", lockout::now()),
        );
    }

    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn vault_path(&self) -> PathBuf {
        self.dir.join(VAULT)
    }

    pub fn exists(&self) -> bool {
        self.vault_path().is_file()
    }

    /// Cheap change detection for the app: one metadata call, no read.
    pub fn modified(&self) -> Option<SystemTime> {
        fs::metadata(self.vault_path())
            .and_then(|meta| meta.modified())
            .ok()
    }

    /// Whether the vault can also be opened with a recovery key. The header
    /// says so in the clear; it needs no password.
    pub fn has_recovery(&self) -> bool {
        self.locked(Self::read)
            .is_ok_and(|file| file.recovery.is_some())
    }

    pub fn lockout_remaining(&self) -> Option<Duration> {
        lockout::remaining(&self.dir)
    }

    /// Runs `work` while holding an exclusive lock on `vault.lokey.lock`. The
    /// lock is a separate file because the vault itself is replaced by rename.
    pub(crate) fn locked<T>(&self, work: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        fs::create_dir_all(&self.dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.dir.join(LOCK))?;
        lock.lock()?;
        // Dropping the handle releases the lock, on every exit path.
        work(self)
    }

    pub(crate) fn read(&self) -> Result<VaultFile> {
        match fs::read(self.vault_path()) {
            Ok(bytes) => VaultFile::parse(&bytes),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(Error::NoVault),
            Err(err) => Err(err.into()),
        }
    }

    /// Writes the vault, exactly as it is on disk, to `dest`. The copy is as
    /// encrypted as the vault itself.
    pub(crate) fn copy_to(&self, dest: &Path) -> Result<()> {
        let bytes = fs::read(self.vault_path())?;
        let mut out = File::create(dest)?;
        out.write_all(&bytes)?;
        out.sync_all()?;
        Ok(())
    }

    pub(crate) fn write(&self, file: &VaultFile, history: History) -> Result<()> {
        let vault = self.vault_path();
        let backup = self.dir.join(BACKUP);
        let suffix: [u8; 8] = crypto::random();
        let temp = self.dir.join(format!(
            "{VAULT}.{}.tmp",
            suffix
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));

        let written = replace(&temp, &vault, &backup, &file.to_json(), history);
        if written.is_err() {
            // Best effort: the temp file holds only ciphertext, but no litter.
            let _ = fs::remove_file(&temp);
        }
        Ok(written?)
    }
}

fn replace(
    temp: &Path,
    vault: &Path,
    backup: &Path,
    bytes: &[u8],
    history: History,
) -> std::io::Result<()> {
    let mut out = File::create(temp)?;
    out.write_all(bytes)?;
    out.sync_all()?;
    drop(out);
    if vault.is_file() {
        fs::copy(vault, backup)?;
    }
    fs::rename(temp, vault)?;
    if history == History::Forget {
        fs::copy(vault, backup)?;
    }
    Ok(())
}

/// Copies each carried file from `old` to `new` through a temporary name, so
/// no file in `new` is ever half-written. Holds a shared lock on the old
/// vault's lock file meanwhile, so a running lokey cannot write mid-copy.
fn copy_from(old: &Path, new: &Path) -> std::io::Result<()> {
    // Opened read-only and never created: the old folder is not written to.
    let lock = File::open(old.join(LOCK)).ok();
    if let Some(lock) = &lock {
        lock.lock_shared()?;
    }
    for name in CARRIED {
        let from = old.join(name);
        if !from.exists() {
            continue;
        }
        let temp = new.join(format!("{name}.migrating"));
        let copied = copy_durably(&from, &temp).and_then(|()| fs::rename(&temp, new.join(name)));
        if copied.is_err() {
            let _ = fs::remove_file(&temp);
        }
        copied?;
    }
    Ok(())
}

fn copy_durably(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::copy(from, to)?;
    OpenOptions::new().write(true).open(to)?.sync_all()
}

fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &str = "velvet otter plumbing ninety";

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let suffix: [u8; 8] = crypto::random();
            let name: String = suffix.iter().map(|b| format!("{b:02x}")).collect();
            let dir = env::temp_dir().join(format!("saltcellar-migration-{name}"));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A `%LOCALAPPDATA%` stand-in holding an old lokey vault, its backup and
    /// its lockout file. Returns the base, old and new folders.
    fn with_old_vault() -> (TempDir, PathBuf, PathBuf) {
        let base = TempDir::new();
        let old = base.0.join(OLD_DIR);
        let new = base.0.join(DIR);
        let store = Store::at(&old);
        store.create(MASTER, false).unwrap();
        // A second write, so `vault.lokey.bak` exists too.
        let (mut session, _) = store.unlock(MASTER).unwrap();
        session.put("default", "API_KEY", "old value").unwrap();
        fs::write(old.join(lockout::FILE), br#"{"fails":2,"until":0}"#).unwrap();
        (base, old, new)
    }

    /// Every file in `dir`, name and bytes, sorted.
    fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy().into_owned();
                (name, fs::read(entry.path()).unwrap_or_default())
            })
            .collect();
        files.sort();
        files
    }

    fn vars(pairs: &[(&'static str, &str)]) -> impl Fn(&'static str) -> Option<OsString> {
        let pairs: Vec<(&'static str, OsString)> = pairs
            .iter()
            .map(|(name, value)| (*name, OsString::from(value)))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn only_the_old_vault_is_copied_byte_for_byte_and_left_untouched() {
        let (_base, old, new) = with_old_vault();
        let before = snapshot(&old);

        let migration = Store::at(&new).adopt(&old);

        assert_eq!(migration, Migration::Copied { from: old.clone() });
        assert_eq!(snapshot(&old), before);
        for name in CARRIED {
            assert_eq!(
                fs::read(new.join(name)).unwrap(),
                fs::read(old.join(name)).unwrap()
            );
        }
        assert!(!new.join(format!("{VAULT}.migrating")).exists());
        let log = fs::read_to_string(new.join(MIGRATION_LOG)).unwrap();
        assert!(log.contains("migrated: copied the vault from"), "{log}");
    }

    #[test]
    fn a_migrated_vault_opens_with_its_master_password() {
        let (_base, old, new) = with_old_vault();
        let store = Store::at(&new);
        store.adopt(&old);

        let (session, _) = store.unlock(MASTER).unwrap();
        assert_eq!(
            session.get("default", "API_KEY").unwrap().value,
            "old value"
        );
    }

    #[test]
    fn when_both_vaults_exist_neither_changes() {
        let (_base, old, new) = with_old_vault();
        Store::at(&new)
            .create("an entirely different phrase", false)
            .unwrap();
        let old_before = snapshot(&old);
        let new_vault = fs::read(new.join(VAULT)).unwrap();

        let migration = Store::at(&new).adopt(&old);

        assert_eq!(migration, Migration::NewVaultExists);
        assert_eq!(snapshot(&old), old_before);
        assert_eq!(fs::read(new.join(VAULT)).unwrap(), new_vault);
        assert!(!new.join(MIGRATION_LOG).exists());
    }

    #[test]
    fn when_neither_vault_exists_nothing_is_created() {
        let base = TempDir::new();
        let new = base.0.join(DIR);

        let migration = Store::at(&new).adopt(&base.0.join(OLD_DIR));

        assert_eq!(migration, Migration::NoOldVault);
        assert!(!new.exists());
        assert!(!base.0.join(OLD_DIR).exists());
    }

    #[test]
    fn a_new_folder_without_a_vault_still_gets_the_copy() {
        let (_base, old, new) = with_old_vault();
        // What an earlier start may leave: the folder and its lock file only.
        fs::create_dir_all(&new).unwrap();
        fs::write(new.join(LOCK), b"").unwrap();

        let migration = Store::at(&new).adopt(&old);

        assert_eq!(migration, Migration::Copied { from: old.clone() });
        assert_eq!(
            fs::read(new.join(VAULT)).unwrap(),
            fs::read(old.join(VAULT)).unwrap()
        );
    }

    #[test]
    fn a_copy_that_fails_partway_leaves_no_vault_and_says_so() {
        let (_base, old, new) = with_old_vault();
        // A backup that cannot be copied stops the copy before the vault.
        fs::remove_file(old.join(BACKUP)).unwrap();
        fs::create_dir(old.join(BACKUP)).unwrap();

        let migration = Store::at(&new).adopt(&old);

        assert!(
            matches!(migration, Migration::Failed { .. }),
            "{migration:?}"
        );
        assert!(!new.join(VAULT).exists());
        let log = fs::read_to_string(new.join(MIGRATION_LOG)).unwrap();
        assert!(
            log.contains("failed: could not copy the vault from"),
            "{log}"
        );

        // Once the problem is gone, the next start copies it.
        fs::remove_dir(old.join(BACKUP)).unwrap();
        assert_eq!(
            Store::at(&new).adopt(&old),
            Migration::Copied { from: old.clone() }
        );
    }

    #[test]
    fn a_folder_variable_skips_the_migration() {
        assert_eq!(
            locate(vars(&[(DIR_VAR, "D:/new"), ("LOCALAPPDATA", "C:/local")])),
            Some(Location::Chosen(PathBuf::from("D:/new")))
        );
        assert_eq!(
            locate(vars(&[
                (OLD_DIR_VAR, "D:/old"),
                ("LOCALAPPDATA", "C:/local")
            ])),
            Some(Location::Chosen(PathBuf::from("D:/old")))
        );
        assert_eq!(
            locate(vars(&[(DIR_VAR, "D:/new"), (OLD_DIR_VAR, "D:/old")])),
            Some(Location::Chosen(PathBuf::from("D:/new")))
        );
    }

    #[test]
    fn the_default_folder_sits_beside_the_old_one() {
        let local = PathBuf::from("C:/local");
        assert_eq!(
            locate(vars(&[("LOCALAPPDATA", "C:/local")])),
            Some(Location::Default {
                dir: local.join("saltcellar"),
                old: local.join("lokey"),
            })
        );
        assert_eq!(locate(vars(&[])), None);
    }
}
