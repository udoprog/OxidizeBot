use std::ffi::{OsStr, OsString};
use std::process::Command;

use anyhow::{bail, Result};

pub(crate) struct Cargo {
    args: Vec<OsString>,
}

impl Cargo {
    pub(crate) fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub(crate) fn with<I>(args: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        let mut cargo = Self::new();
        cargo.args(args);
        cargo
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
        let mut command = Command::new("cargo");
        let mut args = Vec::new();

        for arg in &self.args {
            command.arg(arg);
            args.push(arg.to_string_lossy());
        }

        println!("cargo {}", args.join(" "));
        let status = command.status()?;

        if !status.success() {
            bail!("failed to run cargo");
        }

        Ok(())
    }
}
