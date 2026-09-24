//! lokey desktop app: a thin Tauri shell over `lokey-core`.
//!
//! The page never sees a key or the vault file. It asks for one value at a
//! time (`reveal`), and copying happens here, so a copied value never passes
//! through the page at all. A watcher thread notices when another process
//! changes the vault and pushes a fresh snapshot, which is how an edit made in
//! a second window appears here within a quarter of a second.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dto;

use std::{
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant, SystemTime},
};

use dto::{Failure, Snapshot, Status};
use lokey_core::{ChangeKind, Error, Session, Store, check_new_password, clipboard};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

/// The vault locks itself after this long without any activity in the window.
const IDLE_LOCK: Duration = Duration::from_secs(5 * 60);
/// How often the watcher checks the vault file's modified time. One metadata
/// call per tick; nothing is read unless the time changed.
const WATCH_EVERY: Duration = Duration::from_millis(250);

struct Inner {
    store: Option<Store>,
    session: Option<Session>,
    /// The vault's modified time as of this process's last read or write.
    seen: Option<SystemTime>,
    last_active: Instant,
    /// Clipboard sequence number of our last copy, cleared on exit if unchanged.
    copied: Option<u32>,
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
fn create(state: State<'_, AppState>, master: String) -> Result<Snapshot, Failure> {
    let mut inner = state.lock();
    inner.store()?.create(&master)?;
    open(&mut inner, &master)
}

#[tauri::command(async)]
fn unlock(state: State<'_, AppState>, master: String) -> Result<Snapshot, Failure> {
    let result = open(&mut state.lock(), &master);
    // The backoff after a wrong password is served before answering, and
    // outside the state lock so the window stays responsive meanwhile.
    if let Err(Failure { wait_ms, .. }) = &result {
        thread::sleep(Duration::from_millis(*wait_ms));
    }
    result
}

fn open(inner: &mut Inner, master: &str) -> Result<Snapshot, Failure> {
    let (session, report) = inner.store()?.unlock(master)?;
    let snapshot = Snapshot::new(&session, &report);
    inner.session = Some(session);
    inner.last_active = Instant::now();
    inner.mark_seen();
    Ok(snapshot)
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
    let (kind, report) = session.put(&project, &key, &value)?;
    let mut snapshot = Snapshot::new(session, &report);
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
    mutate(&state, |session| session.delete_project(&project).map(|(_, r)| r))
}

#[tauri::command(async)]
fn truncate(state: State<'_, AppState>) -> Result<Snapshot, Failure> {
    mutate(&state, |session| session.truncate().map(|(_, r)| r))
}

#[tauri::command(async)]
fn change_master(state: State<'_, AppState>, new_master: String) -> Result<(), Failure> {
    let mut inner = state.lock();
    let changed = inner.session()?.change_master(&new_master);
    inner.mark_seen();
    changed.map_err(Failure::from)
}

/// Runs a change and returns the new snapshot.
fn mutate(
    state: &State<'_, AppState>,
    change: impl FnOnce(&mut Session) -> lokey_core::Result<lokey_core::Report>,
) -> Result<Snapshot, Failure> {
    let mut inner = state.lock();
    let session = inner.session()?;
    let outcome = change(session).map(|report| Snapshot::new(session, &report));
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
            Ok(report) => {
                let snapshot = Snapshot::new(session, &report);
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
            lock,
            touch,
            reveal,
            copy,
            save,
            delete_key,
            delete_project,
            truncate,
            change_master,
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
