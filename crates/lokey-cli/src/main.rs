//! `lokey`: the terminal front-end. All vault logic lives in `lokey-core`;
//! this crate parses arguments, prompts, and prints.

mod args;
mod commands;
mod import;
mod prompt;

use std::{env, io, process::ExitCode, thread};

use lokey_core::Error;

pub enum Failure {
    /// The command line itself was wrong. Exit code 2.
    Usage(String),
    Message(String),
    Core(Error),
}

impl From<Error> for Failure {
    fn from(err: Error) -> Self {
        Self::Core(err)
    }
}

impl From<io::Error> for Failure {
    fn from(err: io::Error) -> Self {
        Self::Core(Error::Io(err))
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    let outcome = args::parse(&argv)
        .map_err(Failure::Usage)
        .and_then(|args| commands::run(&args));
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Usage(message)) => {
            eprintln!("lokey: {message}");
            eprintln!("run `lokey help` to see the commands");
            ExitCode::from(2)
        }
        Err(Failure::Message(message)) => {
            eprintln!("lokey: {message}");
            ExitCode::FAILURE
        }
        Err(Failure::Core(err)) => {
            // The backoff after a wrong password is served here, before the
            // answer, so a script guessing through the CLI is slowed down.
            if let Error::WrongPassword { wait, .. } = &err {
                thread::sleep(*wait);
            }
            eprintln!("lokey: {}", describe(&err));
            ExitCode::FAILURE
        }
    }
}

fn describe(err: &Error) -> String {
    match err {
        Error::NoVault => "no vault yet. create one with:  lokey init".into(),
        Error::VaultExists => {
            "a vault already exists. change its passwords with:  lokey passwd".into()
        }
        other => other.to_string(),
    }
}
