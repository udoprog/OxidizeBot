use anyhow::{anyhow, Context as _, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const URL: &str = "https://setbac.tv";

/// Construct a Windows version information.
fn file_version() -> Option<(String, u64)> {
    let version = match env::var("OXIDIZE_FILE_VERSION") {
        Ok(version) => version,
        Err(_) => return None,
    };

    let mut info = 0u64;

    let mut it = version.split('.');

    info |= it.next()?.parse().unwrap_or(0) << 48;
    info |= it.next()?.parse().unwrap_or(0) << 32;
    info |= it.next()?.parse().unwrap_or(0) << 16;
    info |= match it.next() {
        Some(n) => n.parse().unwrap_or(0),
        None => 0,
    };

    Some((version, info))
}

fn main() -> Result<()> {
    if cfg!(target_os = "windows") {
        use winres::VersionInfo::{FILEVERSION, PRODUCTVERSION};

        let mut res = winres::WindowsResource::new();
        res.set_icon("res/icon.ico");

        if let Some((version, info)) = file_version() {
            res.set("FileVersion", &version);
            res.set("ProductVersion", &version);
            res.set_version_info(FILEVERSION, info);
            res.set_version_info(PRODUCTVERSION, info);
        }

        res.compile().context("compiling resorces")?;
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| anyhow!("missing: OUT_DIR"))?);

    let version;
    let user_agent;

    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()?;

    let rev = std::str::from_utf8(&output.stdout)?.trim();

    if let Ok(oxidize_version) = env::var("OXIDIZE_VERSION") {
        version = oxidize_version;
        user_agent = format!("OxidizeBot/{version} (git {rev}; +{URL})",);
    } else {
        version = format!("git-{rev}");
        user_agent = format!("OxidizeBot/0 (git {rev}; +{URL})");
    }

    fs::write(out_dir.join("version.txt"), &version).context("writing version.txt")?;
    fs::write(out_dir.join("user_agent.txt"), user_agent).context("writing user_agent.txt")?;
    Ok(())
}
