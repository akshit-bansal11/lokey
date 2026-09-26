//! lokey desktop app: a thin Tauri shell over `lokey-core`.
//!
//! The page never sees a key or the vault file. It asks for one value at a
//! time (`reveal`), and copying happens here, so a copied value never passes
//! through the page at all. A watcher thread notices when another process
//! changes the vault and pushes a fresh snapshot, which is how an edit made in
//! a second window appears here within a quarter of a second.
//!
//! The one secret the page does receive is a new recovery key, once, so the
//! owner can write it down.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dto;
mod file_dialog;

use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard, mpsc},
    thread,
    time::{Duration, Instant, SystemTime},
};

use dto::{Failure, Opened, Snapshot, Status};
use lokey_core::{
    ChangeKind, Error, RecoveryKey, Report, Session, Store, check_new_password, clipboard,
};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewWindow};

/// The vault locks itself after this long without any activity in the window.
const IDLE_LOCK: Duration = Duration::from_secs(5 * 60);
/// How often the watcher checks the vault file's modified time. One metadata
/// call per tick; nothing is read unless the time changed.
const WATCH_EVERY: Duration = Duration::from_millis(250);
/// What the Save dialog suggests for a backup's file name.
const BACKUP_NAME: &str = "lokey-backup.lokey";

struct Inner {
    store: Option<Store>,
    session: Option<Session>,
    /// The vault's modified time as of this process's last read or write.
    seen: Option<SystemTime>,
    last_active: Instant,
    /// Clipboard sequence number of our last copy, cleared on exit if unchanged.
    copied: Option<u32>,
    /// The backup chosen in the Open dialog, waiting for its password. Kept
    /// here so the page never hands this side a path to read.
    restore_from: Option<PathBuf>,
}

struct AppState(Mutex<Inner>);

impl AppState {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A poisoned lock only means a command panicked; the data is still usable.
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Inner {
    fn store(&self) -> Result<&Store, Failure> {
        self.store.as_ref().ok_or_else(Failure::no_data_folder)
    }

    fn session(&mut self) -> Result<&mut Session, Failure> {
        self.last_active = Instant::now();
        self.session.as_mut().ok_or_else(Failure::locked)
    }

    fn mark_seen(&mut self) {
        self.seen = self.store.as_ref().and_then(Store::modified);
    }

    /// Makes `session` the open vault and returns what the page shows.
    fn enter(&mut self, session: Session, report: &Report) -> Snapshot {
        let snapshot = Snapshot::with_report(&session, report);
        self.session = Some(session);
        self.last_active = Instant::now();
        self.mark_seen();
        snapshot
    }
}

/// The backoff after a wrong password or recovery key is served before
/// answering. Callers drop the state lock first, so the window stays
/// responsive meanwhile.
fn served<T>(result: Result<T, Failure>) -> Result<T, Failure> {
    if let Err(Failure { wait_ms, .. }) = &result {
        thread::sleep(Duration::from_millis(*wait_ms));
    }
    result
}

/// Asks for the master password again before anything that adds a way into
/// the vault or takes a copy of it away, so a vault left unlocked cannot be
/// given a password or recovery key someone else knows, or be copied off.
fn verified(state: &State<'_, AppState>, master: &str) -> Result<(), Failure> {
    let result = state
        .lock()
        .session()
        .and_then(|session| Ok(session.verify_master(master)?));
    served(result)
}

fn shown(recovery_key: RecoveryKey) -> String {
    recovery_key.as_str().to_owned()
}

/// Runs `work` on the main thread, where Windows dialogs belong, and waits.
fn on_main_thread<T: Send + 'static>(
    app: &AppHandle,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, Failure> {
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(work());
    })
    .map_err(|err| Failure::io(err.to_string()))?;
    receiver
        .recv()
        .map_err(|_| Failure::io("the file dialog closed without an answer"))
}

#[tauri::command(async)]
fn status(state: State<'_, AppState>) -> Status {
    let inner = state.lock();
    match &inner.store {
        Some(store) => Status {
            vault_path: store.vault_path().display().to_string(),
            exists: store.exists(),
            unlocked: inner.session.is_some(),
            lockout_secs: store.lockout_remaining().map_or(0, |left| left.as_secs()),
            recovery: store.has_recovery(),
        },
        None => Status::default(),
    }
}

/// Checks a new password against the policy without deriving anything, so the
/// form can say what is wrong on blur rather than after a slow submit.
#[tauri::command]
fn check_password(label: String, password: String) -> Result<(), Failure> {
    Ok(check_new_password(&label, &password)?)
}

#[tauri::command(async)]
fn create(state: State<'_, AppState>, master: String, recovery: bool) -> Result<Opened, Failure> {
    let mut inner = state.lock();
    let recovery_key = inner.store()?.create(&master, recovery)?;
    let (session, report) = inner.store()?.unlock(&master)?;
    Ok(Opened {
        snapshot: inner.enter(session, &report),
        recovery_key: recovery_key.map(shown),
    })
}

#[tauri::command(async)]
fn unlock(state: State<'_, AppState>, master: String) -> Result<Snapshot, Failure> {
    let result = open(&mut state.lock(), &master);
    served(result)
}

fn open(inner: &mut Inner, master: &str) -> Result<Snapshot, Failure> {
    let (session, report) = inner.store()?.unlock(master)?;
    Ok(inner.enter(session, &report))
}

/// Opens the vault with its recovery key and sets a new master password.
#[tauri::command(async)]
fn recover(
    state: State<'_, AppState>,
    recovery_key: String,
    new_master: String,
) -> Result<Opened, Failure> {
    let result = recover_with(&mut state.lock(), &recovery_key, &new_master);
    served(result)
}

fn recover_with(
    inner: &mut Inner,
    recovery_key: &str,
    new_master: &str,
) -> Result<Opened, Failure> {
    let (session, fresh) = inner.store()?.recover(recovery_key, new_master)?;
    Ok(Opened {
        snapshot: inner.enter(session, &Report::default()),
        recovery_key: Some(shown(fresh)),
    })
}

/// Shows the Open dialog for a backup. Returns the chosen path to display,
/// or `None` when the person cancelled.
#[tauri::command(async)]
fn choose_backup(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<String>, Failure> {
    let chosen = on_main_thread(&app, move || file_dialog::open(&window))?;
    let display = chosen.as_ref().map(|path| path.display().to_string());
    state.lock().restore_from = chosen;
    Ok(display)
}

/// Restores the backup chosen in `choose_backup`, if its password opens it.
#[tauri::command(async)]
fn restore(state: State<'_, AppState>, master: String) -> Result<Snapshot, Failure> {
    let result = restore_chosen(&mut state.lock(), &master);
    served(result)
}

fn restore_chosen(inner: &mut Inner, master: &str) -> Result<Snapshot, Failure> {
    let backup = inner
        .restore_from
        .clone()
        .ok_or_else(|| Failure::io("choose a backup file first"))?;
    let (session, report) = inner.store()?.restore(&backup, master)?;
    inner.restore_from = None;
    Ok(inner.enter(session, &report))
}

/// Asks for the master password again, then where to save, then writes the
/// encrypted backup. Returns the path written, or `None` when cancelled.
#[tauri::command(async)]
fn export_backup(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    master: String,
) -> Result<Option<String>, Failure> {
    verified(&state, &master)?;
    let Some(dest) = on_main_thread(&app, move || file_dialog::save(&window, BACKUP_NAME))? else {
        return Ok(None);
    };
    state.lock().session()?.export(&dest)?;
    Ok(Some(dest.display().to_string()))
}

#[tauri::command(async)]
fn lock(state: State<'_, AppState>) {
    state.lock().session = None;
}

#[tauri::command(async)]
fn touch(state: State<'_, AppState>) {
    state.lock().last_active = Instant::now();
}

#[tauri::command(async)]
fn reveal(state: State<'_, AppState>, project: String, key: String) -> Result<String, Failure> {
    let mut inner = state.lock();
    Ok(inner.session()?.get(&project, &key)?.value.clone())
}

/// Copies a value and schedules the clear. Returns the clear delay in seconds.
#[tauri::command(async)]
fn copy(state: State<'_, AppState>, project: String, key: String) -> Result<u64, Failure> {
    let mut inner = state.lock();
    let sequence = clipboard::copy_secret(&inner.session()?.get(&project, &key)?.value)?;
    inner.copied = Some(sequence);
    thread::spawn(move || {
        thread::sleep(clipboard::CLEAR_AFTER);
        // Nothing to report if this fails: the value was ours to clear, and a
        // failure here means another program holds the clipboard right now.
        let _ = clipboard::clear_if_unchanged(sequence);
    });
    Ok(clipboard::CLEAR_AFTER.as_secs())
}

/// Adds a key or replaces its value. Returns whether it was new.
#[tauri::command(async)]
fn save(
    state: State<'_, AppState>,
    project: String,
    key: String,
    value: String,
) -> Result<Snapshot, Failure> {
    let mut inner = state.lock();
    let session = inner.session()?;
    let kind = session.put(&project, &key, &value)?;
    let mut snapshot = Snapshot::new(session);
    snapshot.saved = Some(kind == ChangeKind::Added);
    inner.mark_seen();
    Ok(snapshot)
}

#[tauri::command(async)]
fn delete_key(
    state: State<'_, AppState>,
    project: String,
    key: String,
) -> Result<Snapshot, Failure> {
    mutate(&state, |session| session.delete_key(&project, &key))
}

#[tauri::command(async)]
fn delete_project(state: State<'_, AppState>, project: String) -> Result<Snapshot, Failure> {
    mutate(&state, |session| session.delete_project(&project).map(|_| ()))
}

#[tauri::command(async)]
fn truncate(state: State<'_, AppState>) -> Result<Snapshot, Failure> {
    mutate(&state, |session| session.truncate().map(|_| ()))
}

/// Changes the master password. Returns the replacement recovery key when the
/// vault has one, since the old one stops working.
#[tauri::command(async)]
fn change_master(
    state: State<'_, AppState>,
    current_master: String,
    new_master: String,
) -> Result<Option<String>, Failure> {
    verified(&state, &current_master)?;
    let mut inner = state.lock();
    let changed = inner.session()?.change_master(&new_master);
    inner.mark_seen();
    Ok(changed?.map(shown))
}

/// Issues a new recovery key (`on`), returned once, or removes it.
#[tauri::command(async)]
fn set_recovery(
    state: State<'_, AppState>,
    master: String,
    on: bool,
) -> Result<Option<String>, Failure> {
    verified(&state, &master)?;
    let mut inner = state.lock();
    let changed = inner.session()?.set_recovery(on);
    inner.mark_seen();
    Ok(changed?.map(shown))
}

/// Runs a change and returns the new snapshot.
fn mutate(
    state: &State<'_, AppState>,
    change: impl FnOnce(&mut Session) -> lokey_core::Result<()>,
) -> Result<Snapshot, Failure> {
    let mut inner = state.lock();
    let session = inner.session()?;
    let outcome = change(session).map(|()| Snapshot::new(session));
    inner.mark_seen();
    outcome.map_err(Failure::from)
}

/// Pushes changes made by other processes into the window, and locks the
/// vault after `IDLE_LOCK` without activity.
fn watch(app: &AppHandle) {
    let mut reported = None;
    loop {
        thread::sleep(WATCH_EVERY);
        let state = app.state::<AppState>();
        let mut inner = state.lock();
        if inner.session.is_none() {
            continue;
        }
        if inner.last_active.elapsed() >= IDLE_LOCK {
            inner.session = None;
            let _ = app.emit("vault-locked", "idle");
            continue;
        }
        let modified = inner.store.as_ref().and_then(Store::modified);
        if modified == inner.seen {
            continue;
        }
        let Some(session) = inner.session.as_mut() else {
            continue;
        };
        match session.refresh() {
            Ok(()) => {
                let snapshot = Snapshot::new(session);
                inner.mark_seen();
                let _ = app.emit("vault-changed", snapshot);
            }
            Err(Error::Stale) => {
                inner.session = None;
                let _ = app.emit("vault-locked", "stale");
            }
            Err(Error::NoVault) => {
                inner.session = None;
                let _ = app.emit("vault-locked", "missing");
            }
            Err(other) => {
                // Probably a write in progress or a scanner holding the file.
                // `seen` stays put so the next tick tries again; the error is
                // reported once per change, not every tick.
                if reported != modified {
                    reported = modified;
                    let _ = app.emit("vault-error", other.to_string());
                }
            }
        }
    }
}

fn main() {
    let app = tauri::Builder::default()
        .manage(AppState(Mutex::new(Inner {
            store: Store::open_default().ok(),
            session: None,
            seen: None,
            last_active: Instant::now(),
            copied: None,
            restore_from: None,
        })))
        .setup(|app| {
            let handle = app.handle().clone();
            thread::spawn(move || watch(&handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            status,
            check_password,
            create,
            unlock,
            recover,
            choose_backup,
            restore,
            export_backup,
            lock,
            touch,
            reveal,
            copy,
            save,
            delete_key,
            delete_project,
            truncate,
            change_master,
            set_recovery,
        ])
        .build(tauri::generate_context!())
        .expect("lokey failed to start");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            let state = handle.state::<AppState>();
            let mut inner = state.lock();
            inner.session = None;
            // Do not leave a copied secret behind when the app closes.
            if let Some(sequence) = inner.copied.take() {
                let _ = clipboard::clear_if_unchanged(sequence);
            }
        }
    });
}
