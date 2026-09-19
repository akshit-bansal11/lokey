//! Everything that reads from the person at the keyboard. Prompts go to the
//! console, never to stdout, so `lokey get key=X --view` output stays clean
//! for scripts.

use std::io::{self, BufRead, Write};

use lokey_core::{MIN_PASSWORD_CHARS, check_new_password};
use zeroize::Zeroizing;

use crate::Failure;

/// Retries allowed for a new password before giving up, so a closed or
/// redirected input cannot spin forever.
const ATTEMPTS: usize = 5;

/// Reads without echo, straight from the console.
pub fn secret(prompt: &str) -> io::Result<Zeroizing<String>> {
    rpassword::prompt_password(prompt).map(Zeroizing::new)
}

/// Asks for a new password twice and checks it against the policy.
/// `different_from` is a password this one must not equal.
pub fn new_password(
    label: &str,
    different_from: Option<&str>,
) -> Result<Zeroizing<String>, Failure> {
    for _ in 0..ATTEMPTS {
        let first = secret(&format!(
            "  {label} password ({MIN_PASSWORD_CHARS}+ characters): "
        ))?;
        if let Err(err) = check_new_password(label, &first) {
            eprintln!("  {err}");
            continue;
        }
        if different_from == Some(first.as_str()) {
            eprintln!("  the master and deletion passwords must be different");
            continue;
        }
        let again = secret(&format!("  {label} password again: "))?;
        if *again == *first {
            return Ok(first);
        }
        eprintln!("  they did not match, try again");
    }
    Err(Failure::Message("no password set; nothing changed".into()))
}

/// A visible line of input, trimmed.
pub fn line(prompt: &str) -> io::Result<String> {
    eprint!("{prompt}");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    Ok(answer.trim().to_string())
}

pub fn confirm(question: &str) -> io::Result<bool> {
    Ok(line(&format!("{question} [y/N] "))?
        .to_lowercase()
        .starts_with('y'))
}
