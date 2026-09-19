//! Password policy, per SECURITY-SPEC SEC-06 (NIST SP 800-63B-4): a password
//! that is the only authenticator needs at least 15 characters, at least 64
//! must be accepted, it is checked against a list of common and breached
//! passwords, and there are no composition rules.
//!
//! The blocklist ships inside the binary because lokey never touches the
//! network. It holds the entries of SecLists' Pwdb top-1,000,000 list (MIT,
//! Daniel Miessler) that are 15 characters or longer; shorter ones are already
//! refused by the length rule.

use crate::{Error, Result};

pub const MIN_PASSWORD_CHARS: usize = 15;
const MAX_PASSWORD_CHARS: usize = 1024;
const BLOCKLIST: &str = include_str!("common-passwords.txt");

pub fn check_new(label: &str, password: &str) -> Result<()> {
    let chars = password.chars().count();
    if chars < MIN_PASSWORD_CHARS {
        return Err(Error::WeakPassword(format!(
            "the {label} password needs at least {MIN_PASSWORD_CHARS} characters \
             (a few random words works well)"
        )));
    }
    if chars > MAX_PASSWORD_CHARS {
        return Err(Error::WeakPassword(format!(
            "the {label} password is longer than {MAX_PASSWORD_CHARS} characters"
        )));
    }
    let lower = password.to_lowercase();
    if BLOCKLIST.lines().any(|line| line == lower) {
        return Err(Error::WeakPassword(format!(
            "the {label} password appears in lists of leaked passwords; choose another"
        )));
    }
    if is_one_char_repeated(&lower) || is_sequence(&lower) {
        return Err(Error::WeakPassword(format!(
            "the {label} password is too predictable; choose another"
        )));
    }
    Ok(())
}

fn is_one_char_repeated(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|first| chars.all(|c| c == first))
}

/// "123456789012345" or "abcdefghijklmnop": every character one step from the last.
fn is_sequence(text: &str) -> bool {
    let codes: Vec<u32> = text.chars().map(u32::from).collect();
    let ascending = codes
        .windows(2)
        .all(|w| w[1] == w[0] + 1 || (w[0] == u32::from('9') && w[1] == u32::from('0')));
    let descending = codes.windows(2).all(|w| w[0] == w[1] + 1);
    ascending || descending
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_new_accepts_long_random_passphrase() {
        assert!(check_new("master", "velvet otter plumbing ninety").is_ok());
    }

    #[test]
    fn check_new_rejects_fourteen_characters() {
        assert!(matches!(
            check_new("master", "abcdefghijk!@#"),
            Err(Error::WeakPassword(_))
        ));
    }

    #[test]
    fn check_new_rejects_blocklisted_password_case_insensitively() {
        assert!(matches!(
            check_new("master", "ASDFGHJKLQWERTYUIOP"),
            Err(Error::WeakPassword(_))
        ));
    }

    #[test]
    fn check_new_rejects_repeated_character() {
        assert!(matches!(
            check_new("master", "aaaaaaaaaaaaaaaa"),
            Err(Error::WeakPassword(_))
        ));
    }

    #[test]
    fn check_new_rejects_digit_sequence() {
        assert!(matches!(
            check_new("master", "1234567890123456"),
            Err(Error::WeakPassword(_))
        ));
    }

    #[test]
    fn blocklist_is_lowercase_and_long() {
        let is_bad = |line: &&str| line.len() < MIN_PASSWORD_CHARS || *line != line.to_lowercase();

        let bad = BLOCKLIST.lines().find(is_bad);

        assert_eq!(bad, None);
    }
}
