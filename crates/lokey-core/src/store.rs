//! Where the vault lives and how it is read and written.
//!
//! The CLI and the app are separate processes sharing one file. Every change
//! is "take the lock, re-read, change, write, release", so two writers can
//! never overwrite each other's change. Writes go to a temporary file that is
//! renamed over the vault, so a crash never leaves a half-written vault.

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use crate::{Error, Result, crypto, format::VaultFile, lockout};

const VAULT: &str = "vault.lokey";
const BACKUP: &str = "vault.lokey.bak";
const LOCK: &str = "vault.lokey.lock";

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
    /// `LOKEY_DIR` if set, otherwise `%LOCALAPPDATA%\lokey` on Windows. Local,
    /// not Roaming, so Windows never syncs the vault to another machine.
    pub fn open_default() -> Result<Self> {
        let dir = env::var_os("LOKEY_DIR")
            .map(PathBuf::from)
            .or_else(|| env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("lokey")))
            .or_else(|| env::var_os("XDG_DATA_HOME").map(|dir| PathBuf::from(dir).join("lokey")))
            .or_else(|| {
                env::var_os("HOME").map(|dir| PathBuf::from(dir).join(".local/share/lokey"))
            })
            .ok_or(Error::Unsupported(
                "cannot find a data folder; set LOKEY_DIR to choose one",
            ))?;
        Ok(Self::at(dir))
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

    /// Values added by `set` and not yet merged. Needs no password.
    pub fn pending(&self) -> Result<usize> {
        self.locked(|store| Ok(store.read()?.inbox.len()))
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
