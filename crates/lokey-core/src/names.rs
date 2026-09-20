//! What a key or project name may look like, and how two names are compared.

use crate::{Error, Result};

pub const DEFAULT_PROJECT: &str = "default";
const MAX_NAME_CHARS: usize = 128;
/// 64 KiB keeps one value from inflating the file that every change re-reads.
pub const MAX_VALUE_BYTES: usize = 64 * 1024;

/// Letters, digits and `_ - . /`, so a name survives every shell unquoted.
/// The ban on `=` is inherited from the removed command line, whose arguments
/// were `key=value`; vaults were written under this rule, so it stays.
pub fn check_name(kind: &str, name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidName(format!("{kind} name is empty")));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(Error::InvalidName(format!(
            "{kind} name is longer than {MAX_NAME_CHARS} characters"
        )));
    }
    let allowed = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/');
    if let Some(bad) = name.chars().find(|c| !allowed(*c)) {
        return Err(Error::InvalidName(format!(
            "{kind} name '{name}' contains '{bad}'; use letters, digits, _ - . /"
        )));
    }
    Ok(())
}

pub fn check_value(value: &str) -> Result<()> {
    if value.len() > MAX_VALUE_BYTES {
        return Err(Error::TooLarge(format!(
            "value is larger than {} KiB",
            MAX_VALUE_BYTES / 1024
        )));
    }
    Ok(())
}

/// Names match case-insensitively, like Windows environment variables.
pub fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_name_accepts_env_style_names() {
        assert!(check_name("key", "DATABASE_URL").is_ok());
        assert!(check_name("project", "web-app/v2.1").is_ok());
    }

    #[test]
    fn check_name_rejects_equals_sign() {
        assert!(matches!(
            check_name("key", "A=B"),
            Err(Error::InvalidName(_))
        ));
    }

    #[test]
    fn check_name_rejects_empty() {
        assert!(matches!(check_name("key", ""), Err(Error::InvalidName(_))));
    }

    #[test]
    fn check_value_rejects_over_64_kib() {
        let big = "x".repeat(MAX_VALUE_BYTES + 1);

        assert!(matches!(check_value(&big), Err(Error::TooLarge(_))));
    }

    #[test]
    fn same_ignores_case() {
        assert!(same("Api_Key", "API_KEY"));
    }
}
