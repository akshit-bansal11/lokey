//! One function per command. Password rules, as the owner specified them:
//! `set` and `import` need no password; everything that reads or changes the
//! vault asks for the master password, every time; `delete` and `truncate`
//! also ask for the deletion password.

use std::{
    io::{self, IsTerminal, Read},
    thread,
};

use lokey_core::{
    ChangeKind, DEFAULT_PROJECT, Entry, Error, Report, Session, Store, check_name, clipboard,
    same_name,
};
use zeroize::Zeroizing;

use crate::{Failure, args::Args, import, prompt};

const HELP: &str = "\
lokey: a local-only encrypted key/value vault. No network, ever.

  lokey set key=NAME value=VALUE [project=P]   save a value (no password needed)
  lokey set key=NAME [project=P]               same, typing the value hidden
  lokey get key=NAME [project=P]               copy it (clipboard clears in 30s)
  lokey get key=NAME [project=P] --view        print it instead
  lokey view [project=P]                       print every key and value
  lokey projects                               list projects and key counts
  lokey delete key=NAME [project=P]            delete one key
  lokey delete project=P                       delete a project and its keys
  lokey truncate                               delete every key
  lokey import [project=P]                     paste KEY=VALUE or key,value; lines

  lokey init                                   create the vault (sets both passwords)
  lokey passwd                                 change the master or deletion password
  lokey status                                 vault location, pending values, lockout
  lokey help | lokey version

Passwords: set and import need none. Everything else asks for the master
password. delete and truncate also ask for the deletion password.
set and import save to project 'default' unless you add project=P. get and
delete without project= find the key in whichever project holds it.
Names: letters, digits and _ - . /";

pub fn run(args: &Args) -> Result<(), Failure> {
    match args.command.as_str() {
        "" | "help" => {
            println!("{HELP}");
            Ok(())
        }
        "version" => {
            println!("lokey {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "init" => init(),
        "set" => set(args),
        "get" => get(args),
        "view" => view(args),
        "projects" => projects(),
        "delete" => delete(args),
        "truncate" => truncate(),
        "import" => import_pairs(args),
        "passwd" => passwd(),
        "status" => status(),
        // Internal: the detached helper `get` starts to clear the clipboard.
        "__clear-clipboard" => clear_clipboard_later(args),
        other => Err(Failure::Usage(format!("unknown command '{other}'"))),
    }
}

fn store() -> Result<Store, Failure> {
    Ok(Store::open_default()?)
}

fn project_of(args: &Args) -> &str {
    args.project.as_deref().unwrap_or(DEFAULT_PROJECT)
}

fn required_key(args: &Args, usage: &str) -> Result<String, Failure> {
    let key = args
        .key
        .clone()
        .ok_or_else(|| Failure::Usage(format!("missing key=NAME. usage: {usage}")))?;
    check_name("key", &key)?;
    Ok(key)
}

/// Prompts for the master password and opens the vault.
fn unlock(store: &Store) -> Result<Session, Failure> {
    if !store.exists() {
        return Err(Error::NoVault.into());
    }
    let master = prompt::secret("master password: ")?;
    let (session, report) = store.unlock(&master)?;
    print_report(&report);
    Ok(session)
}

/// What arrived through password-less `set` since the vault was last opened.
/// Printed to stderr so it never mixes into `get --view` output.
fn print_report(report: &Report) {
    for change in &report.changes {
        let what = match change.kind {
            ChangeKind::Added => "added",
            ChangeKind::Replaced => "replaced",
        };
        eprintln!(
            "  {what} since last unlock: {} · {}",
            change.project, change.key
        );
    }
    if report.rejected > 0 {
        eprintln!(
            "  warning: {} pending value(s) could not be opened and were discarded",
            report.rejected
        );
    }
    if report.header_restored {
        eprintln!(
            "  warning: the vault's public key had been replaced and was restored.\n  \
             Values added with `set` while it was replaced may have been readable by\n  \
             whoever replaced it. Change those secrets at their source."
        );
    }
}

/// Finds a key. Without `project=`, a key that exists in exactly one project
/// is found wherever it is; one in several projects asks you to choose.
fn find<'a>(session: &'a Session, key: &str, project: Option<&str>) -> Result<&'a Entry, Failure> {
    if let Some(project) = project {
        return Ok(session.get(project, key)?);
    }
    let matches: Vec<&Entry> = session
        .entries()
        .iter()
        .filter(|entry| same_name(&entry.key, key))
        .collect();
    match matches.as_slice() {
        [] => Err(Failure::Message(format!("no key named {key} in any project"))),
        [only] => Ok(only),
        several => {
            let names: Vec<&str> = several.iter().map(|e| e.project.as_str()).collect();
            Err(Failure::Message(format!(
                "{key} exists in {}. add project=NAME",
                names.join(", ")
            )))
        }
    }
}

fn init() -> Result<(), Failure> {
    let store = store()?;
    if store.exists() {
        return Err(Error::VaultExists.into());
    }
    eprintln!("Creating a vault at {}", store.vault_path().display());
    eprintln!();
    eprintln!("  master password    unlocks everything: get, view, delete, truncate");
    eprintln!("  deletion password  also required to delete keys");
    eprintln!();
    eprintln!("There is no recovery. Forget the master password and the vault is lost.");
    eprintln!("Tip: a few random words make a strong, memorable password.");
    eprintln!();
    let master = prompt::new_password("master", None)?;
    let deletion = prompt::new_password("deletion", Some(master.as_str()))?;
    store.create(&master, &deletion)?;
    println!("vault created. add a key:  lokey set key=NAME value=VALUE");
    Ok(())
}

fn set(args: &Args) -> Result<(), Failure> {
    let key = required_key(args, "lokey set key=NAME value=VALUE")?;
    let project = project_of(args);
    check_name("project", project)?;
    let store = store()?;
    if !store.exists() {
        return Err(Error::NoVault.into());
    }
    let value = match &args.value {
        Some(value) => {
            eprintln!(
                "note: values typed on the command line are kept in your shell history.\n      \
                 leave out value= to type it hidden instead."
            );
            Zeroizing::new(value.clone())
        }
        None => prompt::secret(&format!("value for {key}: "))?,
    };
    store.set(project, &key, &value)?;
    println!("saved {key} in {project}");
    Ok(())
}

fn get(args: &Args) -> Result<(), Failure> {
    let key = required_key(args, "lokey get key=NAME [--view]")?;
    let store = store()?;
    let session = unlock(&store)?;
    let entry = find(&session, &key, args.project.as_deref())?;
    if args.view {
        println!("{}", entry.value);
        return Ok(());
    }
    let sequence = clipboard::copy_secret(&entry.value)?;
    if let Err(err) = schedule_clipboard_clear(sequence) {
        eprintln!("warning: could not schedule the clipboard clear ({err}); clear it yourself");
    }
    eprintln!(
        "copied {} · clipboard clears in {}s",
        entry.key,
        clipboard::CLEAR_AFTER.as_secs()
    );
    Ok(())
}

fn view(args: &Args) -> Result<(), Failure> {
    let store = store()?;
    let session = unlock(&store)?;
    let mut rows: Vec<&Entry> = session
        .entries()
        .iter()
        .filter(|entry| {
            args.project
                .as_deref()
                .is_none_or(|project| same_name(&entry.project, project))
        })
        .collect();
    rows.sort_by_key(|entry| {
        (
            !same_name(&entry.project, DEFAULT_PROJECT),
            entry.project.to_lowercase(),
            entry.key.to_lowercase(),
        )
    });
    if rows.is_empty() {
        println!("no keys yet. add one:  lokey set key=NAME value=VALUE");
        return Ok(());
    }
    let show_project = args.project.is_none();
    let key_width = rows
        .iter()
        .map(|e| e.key.chars().count())
        .max()
        .unwrap_or(3)
        .max(3);
    let project_width = rows
        .iter()
        .map(|e| e.project.chars().count())
        .max()
        .unwrap_or(7)
        .max(7);
    let number_width = rows.len().to_string().len().max(1);
    let header = if show_project {
        format!(
            "{:>number_width$}  {:project_width$}  {:key_width$}  VALUE",
            "#", "PROJECT", "KEY"
        )
    } else {
        format!("{:>number_width$}  {:key_width$}  VALUE", "#", "KEY")
    };
    println!("{header}");
    for (index, entry) in rows.iter().enumerate() {
        // Keep one entry per line even when a value spans several.
        let value = entry.value.replace('\r', "").replace('\n', "\\n");
        if show_project {
            println!(
                "{:>number_width$}  {:project_width$}  {:key_width$}  {value}",
                index + 1,
                entry.project,
                entry.key
            );
        } else {
            println!(
                "{:>number_width$}  {:key_width$}  {value}",
                index + 1,
                entry.key
            );
        }
    }
    Ok(())
}

fn projects() -> Result<(), Failure> {
    let store = store()?;
    let session = unlock(&store)?;
    let projects = session.projects();
    if projects.is_empty() {
        println!("no keys yet, so no projects. keys go to 'default' unless you add project=NAME");
        return Ok(());
    }
    let width = projects
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(7)
        .max(7);
    println!("{:width$}  KEYS", "PROJECT");
    for (name, count) in projects {
        println!("{name:width$}  {count}");
    }
    Ok(())
}

fn delete(args: &Args) -> Result<(), Failure> {
    let store = store()?;
    match (&args.key, &args.project) {
        (Some(key), project) => {
            check_name("key", key)?;
            let mut session = unlock(&store)?;
            let entry = find(&session, key, project.as_deref())?;
            let (project, key) = (entry.project.clone(), entry.key.clone());
            let deletion = prompt::secret("deletion password: ")?;
            let report = session.delete_key(&deletion, &project, &key)?;
            print_report(&report);
            println!("deleted {key} from {project}");
        }
        (None, Some(project)) => {
            let mut session = unlock(&store)?;
            let deletion = prompt::secret("deletion password: ")?;
            let (removed, report) = session.delete_project(&deletion, project)?;
            print_report(&report);
            println!("deleted project {project} and its {removed} key(s)");
        }
        (None, None) => {
            return Err(Failure::Usage(
                "say what to delete: key=NAME, or project=NAME for a whole project".into(),
            ));
        }
    }
    Ok(())
}

fn truncate() -> Result<(), Failure> {
    let store = store()?;
    let mut session = unlock(&store)?;
    let count = session.entries().len();
    eprintln!(
        "this deletes all {count} key(s) in {} project(s). the vault and its passwords stay.",
        session.projects().len()
    );
    let deletion = prompt::secret("deletion password: ")?;
    let (removed, report) = session.truncate(&deletion)?;
    print_report(&report);
    println!("deleted {removed} key(s). the vault is empty");
    Ok(())
}

fn import_pairs(args: &Args) -> Result<(), Failure> {
    let project = project_of(args);
    check_name("project", project)?;
    let store = store()?;
    if !store.exists() {
        return Err(Error::NoVault.into());
    }
    let text = read_paste()?;
    let parsed = import::parse(&text);
    if !parsed.bad_lines.is_empty() {
        let lines: Vec<String> = parsed.bad_lines.iter().map(ToString::to_string).collect();
        return Err(Failure::Message(format!(
            "line(s) {} are not KEY=VALUE or key,value pairs. nothing was imported",
            lines.join(", ")
        )));
    }
    if parsed.pairs.is_empty() {
        return Err(Failure::Message("nothing to import".into()));
    }
    let pairs: Vec<(&str, &str)> = parsed
        .pairs
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    store.set_many(project, &pairs)?;
    println!("imported {} key(s) into {project}", pairs.len());
    Ok(())
}

/// Reads everything up to end of input, from a pipe (`Get-Content .env |
/// lokey import`) or a terminal. Stopping at a blank line would leave the rest
/// of a paste in the console for the shell to run, secrets and all.
fn read_paste() -> Result<Zeroizing<String>, Failure> {
    if io::stdin().is_terminal() {
        eprintln!(
            "paste KEY=VALUE or key,value; lines, then finish with Ctrl+Z and Enter \
             (Ctrl+D in Git Bash):"
        );
    }
    let mut text = Zeroizing::new(String::new());
    io::stdin().read_to_string(&mut text)?;
    Ok(text)
}

fn passwd() -> Result<(), Failure> {
    let store = store()?;
    let mut session = unlock(&store)?;
    let new_master = if prompt::confirm("change the master password?")? {
        Some(prompt::new_password("new master", None)?)
    } else {
        None
    };
    let new_deletion = if prompt::confirm("change the deletion password?")? {
        let current = prompt::secret("current deletion password: ")?;
        let next = prompt::new_password("new deletion", new_master.as_deref().map(|m| m.as_str()))?;
        Some((current, next))
    } else {
        None
    };
    if new_master.is_none() && new_deletion.is_none() {
        println!("nothing changed");
        return Ok(());
    }
    session.change_passwords(
        new_master.as_deref().map(|m| m.as_str()),
        new_deletion
            .as_ref()
            .map(|(current, next)| (current.as_str(), next.as_str())),
    )?;
    println!("passwords changed. the open app, if any, will ask to unlock again");
    Ok(())
}

fn status() -> Result<(), Failure> {
    let store = store()?;
    println!("vault     {}", store.vault_path().display());
    if !store.exists() {
        println!("state     no vault yet. create one with:  lokey init");
        return Ok(());
    }
    println!(
        "pending   {} value(s) added with set, merged at next unlock",
        store.pending()?
    );
    match store.lockout_remaining() {
        Some(left) => println!(
            "lockout   locked for {}m {}s",
            left.as_secs() / 60,
            left.as_secs() % 60
        ),
        None => println!("lockout   none"),
    }
    Ok(())
}

/// Starts a detached copy of this program that clears the clipboard after
/// 30 seconds. It receives only the clipboard sequence number, never the value.
#[cfg(windows)]
fn schedule_clipboard_clear(sequence: u32) -> io::Result<()> {
    use std::{os::windows::process::CommandExt, process::Command, process::Stdio};
    use windows_sys::Win32::System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, DETACHED_PROCESS,
    };

    Command::new(std::env::current_exe()?)
        .arg("__clear-clipboard")
        .arg(sequence.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
        .spawn()
        .map(drop)
}

#[cfg(not(windows))]
fn schedule_clipboard_clear(_sequence: u32) -> io::Result<()> {
    Ok(())
}

fn clear_clipboard_later(args: &Args) -> Result<(), Failure> {
    let sequence: u32 = args
        .operand
        .as_deref()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| Failure::Usage("internal command needs a sequence number".into()))?;
    thread::sleep(clipboard::CLEAR_AFTER);
    clipboard::clear_if_unchanged(sequence)?;
    Ok(())
}
