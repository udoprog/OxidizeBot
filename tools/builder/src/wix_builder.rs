use std::env::consts::EXE_EXTENSION;
use std::env::{self, consts};
use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context, Result};

use crate::command::Command;
use crate::Release;

pub(crate) struct WixBuilder {
    candle_bin: PathBuf,
    light_bin: PathBuf,
    wixobj_path: PathBuf,
    installer_path: PathBuf,
}

impl WixBuilder {
    /// Construct a new WIX builder.
    pub(crate) fn new(out: &Path, release: &Release) -> Result<Self> {
        let wix_env = env::var_os("WIX").context("missing environment: WIX")?;
        let wix_bin = PathBuf::from(wix_env).join("bin");

        ensure!(wix_bin.is_dir(), "missing: {}", wix_bin.display());

        let candle_bin = wix_bin.join("candle").with_extension(EXE_EXTENSION);
        ensure!(candle_bin.is_file(), "missing: {}", candle_bin.display());

        let light_bin = wix_bin.join("light").with_extension(EXE_EXTENSION);
        ensure!(light_bin.is_file(), "missing: {}", light_bin.display());

        let base = format!(
            "oxidize-{release}-{os}-{arch}",
            os = consts::OS,
            arch = consts::ARCH
        );

        let wixobj_path = out.join(format!("{base}.wixobj"));
        let installer_path = out.join(format!("{base}.msi"));

        Ok(Self {
            candle_bin,
            light_bin,
            wixobj_path,
            installer_path,
        })
    }

    pub(crate) fn build(&self, source: &Path, file_version: &str) -> Result<()> {
        if self.wixobj_path.is_file() {
            return Ok(());
        }

        let platform = match consts::ARCH {
            "x86_64" => "x64",
            "x86" => "x86",
            arch => bail!("Unsupported arch: {arch}"),
        };

        let mut command = Command::new(&self.candle_bin);

        command
            .arg(format!("-dVersion={}", file_version))
            .arg(format!("-dPlatform={}", platform))
            .args(["-ext", "WixUtilExtension"])
            .arg("-o")
            .arg(&self.wixobj_path)
            .arg(source)
            .run()?;

        Ok(())
    }

    /// Link the current project.
    pub(crate) fn link(&self) -> Result<()> {
        if !self.wixobj_path.is_file() {
            bail!("missing: {}", self.wixobj_path.display());
        }

        if self.installer_path.is_file() {
            return Ok(());
        }

        let mut command = Command::new(&self.light_bin);

        command
            .arg("-spdb")
            .args(["-ext", "WixUIExtension"])
            .args(["-ext", "WixUtilExtension"])
            .arg("-cultures:en-us")
            .arg(&self.wixobj_path)
            .arg("-out")
            .arg(&self.installer_path)
            .run()?;

        Ok(())
    }
}
