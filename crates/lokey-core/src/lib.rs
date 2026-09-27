//! lokey-core: the only code that touches cryptography or the vault file.
//!
//! The desktop app is a thin front-end over this crate. The vault logic lives
//! here so it can be tested without a window.

pub mod clipboard;
mod crypto;
mod error;
mod format;
mod generator;
mod lockout;
mod names;
mod password;
mod store;
mod vault;

pub use error::{Error, Result};
pub use generator::{Charsets, MAX_LENGTH, MIN_LENGTH, generate as generate_password};
pub use password::check_new as check_new_password;
pub use store::Store;
pub use vault::{Change, ChangeKind, Entry, RecoveryKey, Report, Session};
