//! Everything that reads from the person at the keyboard. Prompts go to the
//! console, never to stdout, so `lokey get key=X --view` output stays clean
//! for scripts.

use std::io::{self, BufRead, IsTerminal, Write};

use lokey_core::{MIN_PASSWORD_CHARS, check_new_password};
use zeroize::Zeroizing;

use crate::Failure;

/// Retries allowed for a new password before giving up, so a closed or
/// redirected input cannot spin forever.
const ATTEMPTS: usize = 5;

/// Reads a password without showing it.
///
/// In a Windows console (PowerShell, cmd, Windows Terminal, VS Code, Git Bash
/// inside Windows Terminal) each character shows as `*`, so typing and pasting
/// are visibly accepted. A mintty window (Git Bash, MSYS2, Cygwin) is not a
/// console: its input arrives on a pipe that only its own `stty` can stop
/// echoing, so that path reads stdin with echo turned off through `stty`.
pub fn secret(prompt: &str) -> io::Result<Zeroizing<String>> {
    if !stdin_is_console() && io::stdin().is_terminal() {
        return secret_from_pty(prompt);
    }
    let config = rpassword::ConfigBuilder::new()
        .password_feedback_mask('*')
        .build();
    rpassword::prompt_password_with_config(prompt, config).map(Zeroizing::new)
}

/// mintty and friends: stdin is a Cygwin/MSYS pty. Refuses rather than
/// reading a password the screen would show.
// ponytail: Ctrl+C while echo is off leaves it off; `stty echo` restores it.
fn secret_from_pty(prompt: &str) -> io::Result<Zeroizing<String>> {
    if !stty("-echo") {
        return Err(io::Error::other(
            "this terminal cannot hide what you type, so lokey will not ask for a \
             password here. run it from PowerShell or Windows Terminal, or as: winpty lokey",
        ));
    }
    eprint!("{prompt}");
    io::stderr().flush()?;
    let mut answer = Zeroizing::new(String::new());
    let read = io::stdin().lock().read_line(&mut answer);
    stty("echo");
    eprintln!();
    read?;
    let end = answer.trim_end_matches(['\r', '\n']).len();
    answer.truncate(end);
    Ok(answer)
}

/// Runs the terminal's own `stty` on our stdin, which it recognises as its pty.
fn stty(setting: &str) -> bool {
    std::process::Command::new("stty")
        .arg(setting)
        .stdin(std::process::Stdio::inherit())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn stdin_is_console() -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Console::GetConsoleMode;

    let mut mode = 0;
    // SAFETY: the handle is our own stdin and `mode` outlives the call.
    unsafe { GetConsoleMode(io::stdin().as_raw_handle(), &mut mode) != 0 }
}

#[cfg(not(windows))]
fn stdin_is_console() -> bool {
    true
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
