use std::path::{Path, PathBuf};

use anyhow::{ensure, Result};

use crate::command::Command;

pub(crate) struct SignTool {
    signtool: PathBuf,
    password: String,
    cert: PathBuf,
}

impl SignTool {
    /// Construct a new signtool signer.
    pub(crate) fn open<S, P, C>(signtool: S, password: P, cert: C) -> Option<Self>
    where
        S: AsRef<Path>,
        P: AsRef<str>,
        C: AsRef<Path>,
    {
        if !signtool.as_ref().is_file() {
            return None;
        }

        Some(Self {
            signtool: signtool.as_ref().into(),
            password: password.as_ref().into(),
            cert: cert.as_ref().into(),
        })
    }

    /// Sign the given path with the specified certificate.
    pub(crate) fn sign(&self, path: &Path, what: &str) -> Result<()> {
        println!("Signing: {}", path.display());
        ensure!(self.cert.is_file(), "missing: {}", self.cert.display());

        Command::new(&self.signtool)
            .args(["sign", "/f"])
            .arg(&self.cert)
            .args(["/d", what])
            .args(["/du", "https://github.com/udoprog/OxidizeBot"])
            .args(["/p", self.password.as_str()])
            .arg(path)
            .run()?;

        Ok(())
    }
}
