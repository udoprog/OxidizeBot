use anyhow::{anyhow, bail, Error};
use regex::Regex;
use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

type Result<T, E = Error> = std::result::Result<T, E>;

/// Calculate an MSI-safe version number.
/// Unfortunately this enforces some unfortunate constraints on the available
/// version range.
///
/// The computed patch component must fit within 65535
fn msi_version(major: u32, minor: u32, patch: u32, pre: Option<u32>) -> Result<String> {
    if patch > 64 {
        bail!("patch version must not be greater than 64: {}", patch);
    }

    let mut last = 999;

    if let Some(pre) = pre {
        if pre >= 999 {
            bail!("patch version must not be greater than 64: {}", patch);
        }

        last = pre;
    }

    last += patch * 1000;
    Ok(format!("{}.{}.{}", major, minor, last))
}

fn cargo(args: &[&str]) -> Result<()> {
    println!("cargo {}", args.join(" "));
    let status = Command::new("cargo").args(args).status()?;

    if !status.success() {
        bail!("failed to run cargo");
    }

    Ok(())
}

fn archive(path: &Path, file: impl AsRef<Path>) -> Result<()> {
    let status = Command::new("7z")
        .arg("a")
        .arg(path)
        .arg(file.as_ref())
        .status()?;

    if !status.success() {
        bail!("failed to run 7z");
    }

    Ok(())
}

fn sign(
    signtool: &Path,
    root: &Path,
    certificate_password: &str,
    path: &Path,
    what: &str,
) -> Result<()> {
    println!("Signing: {}", path.display());

    let cert = root.join("bot/res/cert.pfx");

    let status = Command::new(signtool)
        .args(&["sign", "/f"])
        .arg(&cert)
        .args(&[
            "/d",
            what,
            "/du",
            "https://github.com/udoprog/OxidizeBot",
            "/p",
            certificate_password,
        ])
        .arg(path)
        .status()?;

    if !status.success() {
        bail!("failed to run signtool");
    }

    Ok(())
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..\\..");

    println!("ROOT: {}", root.display());

    let version = env::var("APPVEYOR_REPO_TAG_NAME")
        .map_err(|_| anyhow!("missing: APPVEYOR_REPO_TAG_NAME"))?;
    let certificate_password = env::var("CERTIFICATE_PASSWORD").ok();
    let signtool = env::var("SIGNTOOL").map(PathBuf::from).ok();

    let version_re = Regex::new(r"^(\d+)\.(\d+)\.(\d+)(-.+\.(\d+))?$")?;

    // is this a release?
    let m = match version_re.captures(&version) {
        Some(m) => m,
        None => {
            println!("Testing...");
            cargo(&["build", "--all"])?;
            cargo(&["test", "--all"])?;
            return Ok(());
        }
    };

    let exe = root.join("target/release/oxidize.exe");
    let wix_dir = root.join("target/wix");

    let major: u32 = str::parse(&m[1])?;
    let minor: u32 = str::parse(&m[2])?;
    let patch: u32 = str::parse(&m[3])?;
    let pre: Option<u32> = m.get(5).map(|s| str::parse(s.as_str())).transpose()?;

    if !exe.is_file() {
        println!("building: {}", exe.display());
        cargo(&["build", "--release", "--bin", "oxidize"])?;
    }

    if let (Some(signtool), Some(certificate_password)) = (&signtool, &certificate_password) {
        sign(
            signtool,
            &root,
            certificate_password,
            &root.join("target/release/oxidize.exe"),
            "OxidizeBot",
        )?;
    }

    if !wix_dir.is_dir() {
        let msi_version = msi_version(major, minor, patch, pre)?;

        cargo(&[
            "wix",
            "-n",
            "oxidize",
            "--install-version",
            &msi_version,
            "--nocapture",
        ])?;
    }

    let mut installers = Vec::new();

    for e in WalkDir::new(&wix_dir) {
        let e = e?;

        if e.path().extension() == Some(OsStr::new("msi")) {
            installers.push(e.path().to_owned());
        }
    }

    if let (Some(tool), Some(certificate_password)) = (&signtool, &certificate_password) {
        for installer in &installers {
            sign(
                tool,
                &root,
                certificate_password,
                installer,
                "OxidizeBot Installer",
            )?;
        }
    }

    for installer in &installers {
        let name = installer
            .file_name()
            .ok_or_else(|| anyhow!("no file name"))?;
        fs::copy(installer, root.join(name))?;
    }

    let zip = root.join(format!("oxidize-{}-windows-x86_64.zip", version));
    archive(&zip, root.join("README.md"))?;
    archive(&zip, root.join("target/release/oxidize.exe"))?;
    Ok(())
}
