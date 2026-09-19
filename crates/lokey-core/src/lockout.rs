//! Backoff and lockout for wrong passwords, shared by the CLI and the app
//! because it lives next to the vault.
//!
//! This stops a person or script hammering the prompt. It is not the real
//! defence against a stolen file: anyone holding `vault.lokey` can skip this
//! entirely, which is why the key derivation is deliberately expensive.

use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const FILE: &str = "lockout.json";
const MAX_FAILS: u32 = 10;
const LOCKOUT: Duration = Duration::from_secs(15 * 60);
const BACKOFF_CAP_SECS: u64 = 30;

#[derive(Default, Serialize, Deserialize)]
struct State {
    fails: u32,
    /// Unix seconds until which every attempt is refused; 0 when not locked.
    until: u64,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn read(dir: &Path) -> State {
    fs::read(dir.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write(dir: &Path, state: &State) -> Result<()> {
    let json = serde_json::to_vec(state).expect("lockout state serialises");
    fs::write(dir.join(FILE), json)?;
    Ok(())
}

pub fn remaining(dir: &Path) -> Option<Duration> {
    let until = read(dir).until;
    let now = now();
    (until > now).then(|| Duration::from_secs(until - now))
}

pub fn check(dir: &Path) -> Result<()> {
    match remaining(dir) {
        Some(remaining) => Err(Error::LockedOut { remaining }),
        None => Ok(()),
    }
}

/// Records one wrong password and returns the error to show for it.
pub fn fail(dir: &Path) -> Error {
    let mut state = read(dir);
    state.fails += 1;
    let outcome = if state.fails >= MAX_FAILS {
        state = State {
            fails: 0,
            until: now() + LOCKOUT.as_secs(),
        };
        Error::LockedOut { remaining: LOCKOUT }
    } else {
        Error::WrongPassword {
            attempts_left: MAX_FAILS - state.fails,
            wait: backoff(state.fails),
        }
    };
    match write(dir, &state) {
        Ok(()) => outcome,
        Err(err) => err,
    }
}

pub fn clear(dir: &Path) -> Result<()> {
    match fs::remove_file(dir.join(FILE)) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
        _ => Ok(()),
    }
}

/// 1s, 2s, 4s ... capped at 30s.
fn backoff(fails: u32) -> Duration {
    let exponent = fails.saturating_sub(1).min(16);
    Duration::from_secs((1u64 << exponent).min(BACKOFF_CAP_SECS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_from_one_second() {
        assert_eq!(backoff(1), Duration::from_secs(1));
        assert_eq!(backoff(4), Duration::from_secs(8));
    }

    #[test]
    fn backoff_is_capped_at_thirty_seconds() {
        assert_eq!(backoff(9), Duration::from_secs(30));
    }
}
