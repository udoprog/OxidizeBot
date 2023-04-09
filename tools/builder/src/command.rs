use std::ffi::{OsStr, OsString};

use anyhow::{bail, Result};

/// Construct a new cargo command.
#[inline]
pub(crate) fn cargo() -> Command {
    Command::new("cargo")
}

pub(crate) struct Command {
    command: OsString,
    args: Vec<OsString>,
}

impl Command {
    pub(crate) fn new<C>(command: C) -> Self
    where
        C: AsRef<OsStr>,
    {
        Self {
            command: command.as_ref().into(),
            args: Vec::new(),
        }
    }

    pub(crate) fn args<I>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_owned());
        }

        self
    }

    pub(crate) fn arg<A>(&mut self, arg: A) -> &mut Self
    where
        A: AsRef<OsStr>,
    {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub(crate) fn run(&mut self) -> Result<()> {
        let mut command = std::process::Command::new(&self.command);
        let mut args = Vec::new();

        for arg in &self.args {
            command.arg(arg);
            args.push(arg.to_string_lossy());
        }

        println!("run: {} {}", self.command.to_string_lossy(), args.join(" "));
        let status = command.status()?;

        if !status.success() {
            bail!("{}: {status}", self.command.to_string_lossy());
        }

        Ok(())
    }
}
