//! End-to-end: runs the real `lokey` binary against a throwaway vault.
//!
//! Only the password-free commands can run here: every other command reads its
//! password from the console, which a test process does not have. What those
//! commands do with the vault is covered by lokey-core's own tests.

use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use lokey_core::Store;

const MASTER: &str = "velvet otter plumbing ninety";
const DELETION: &str = "quiet granite lantern forty";

struct Vault(PathBuf);

impl Vault {
    /// An empty directory, optionally with a vault already created in it.
    fn new(create: bool) -> Self {
        let dir = env::temp_dir().join(format!(
            "lokey-cli-test-{}-{}",
            std::process::id(),
            unique()
        ));
        fs::create_dir_all(&dir).unwrap();
        if create {
            Store::at(&dir).create(MASTER, DELETION).unwrap();
        }
        Self(dir)
    }

    fn lokey(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lokey"))
            .args(args)
            .env("LOKEY_DIR", &self.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // A command that never reads stdin may exit first; a broken pipe then
        // is expected and says nothing about the command.
        let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
        child.wait_with_output().unwrap()
    }

    fn value(&self, project: &str, key: &str) -> String {
        let (session, _) = Store::at(&self.0).unlock(MASTER).unwrap();
        session.get(project, key).unwrap().value.clone()
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Distinct per call, so parallel tests never share a directory.
fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn set_stores_a_value_readable_with_the_master_password() {
    let vault = Vault::new(true);

    let out = vault.lokey(&["set", "key=API_KEY", "value=sk-123", "project=web"], "");

    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(vault.value("web", "API_KEY"), "sk-123");
}

#[test]
fn set_with_inline_value_warns_about_shell_history() {
    let vault = Vault::new(true);

    let out = vault.lokey(&["set", "key=A", "value=1"], "");

    assert!(text(&out.stderr).contains("shell history"));
}

#[test]
fn import_reads_dotenv_lines_from_a_pipe() {
    let vault = Vault::new(true);

    let out = vault.lokey(
        &["import", "project=api"],
        "DB=postgres://x\nexport TOKEN=\"t 1\"\n",
    );

    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(vault.value("api", "TOKEN"), "t 1");
}

#[test]
fn import_with_a_bad_line_imports_nothing() {
    let vault = Vault::new(true);

    let out = vault.lokey(&["import"], "GOOD=1\nnot a pair\n");

    assert!(!out.status.success());
    assert_eq!(Store::at(&vault.0).pending().unwrap(), 0);
}

#[test]
fn set_without_a_vault_says_how_to_create_one() {
    let vault = Vault::new(false);

    let out = vault.lokey(&["set", "key=A", "value=1"], "");

    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("lokey init"));
}

#[test]
fn set_rejects_a_key_name_with_a_space() {
    let vault = Vault::new(true);

    let out = vault.lokey(&["set", "key=MY KEY", "value=1"], "");

    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn unknown_command_exits_with_usage_code() {
    let vault = Vault::new(true);

    let out = vault.lokey(&["frobnicate"], "");

    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn status_reports_pending_values_without_a_password() {
    let vault = Vault::new(true);
    vault.lokey(&["set", "key=A", "value=1"], "");

    let out = vault.lokey(&["status"], "");

    assert!(text(&out.stdout).contains("pending   1 value(s)"));
}

#[test]
fn version_prints_the_crate_version() {
    let vault = Vault::new(false);

    let out = vault.lokey(&["--version"], "");

    assert_eq!(
        text(&out.stdout).trim(),
        format!("lokey {}", env!("CARGO_PKG_VERSION"))
    );
}
