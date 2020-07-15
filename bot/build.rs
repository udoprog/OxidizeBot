use anyhow::{anyhow, Context as _, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const URL: &str = "https://setbac.tv";

// This code exercises the surface area that we expect of the std Backtrace
// type. If the current toolchain is able to compile it, we go ahead and use
// backtrace in oxidize.
//
// Copied from: https://github.com/dtolnay/anyhow/blob/master/build.rs
// Under the MIT license.
const PROBE: &str = r#"
    #![feature(backtrace)]
    #![allow(dead_code)]

    use std::backtrace::{Backtrace, BacktraceStatus};
    use std::error::Error;
    use std::fmt::{self, Display};

    #[derive(Debug)]
    struct E;

    impl Display for E {
        fn fmt(&self, _formatter: &mut fmt::Formatter) -> fmt::Result {
            unimplemented!()
        }
    }

    impl Error for E {
        fn backtrace(&self) -> Option<&Backtrace> {
            let backtrace = Backtrace::capture();
            match backtrace.status() {
                BacktraceStatus::Captured | BacktraceStatus::Disabled | _ => {}
            }
            unimplemented!()
        }
    }
"#;

fn backtrace_compile_probe() -> Option<ExitStatus> {
    let rustc = env::var_os("RUSTC")?;
    let out_dir = env::var_os("OUT_DIR")?;
    let probe_file = Path::new(&out_dir).join("probe.rs");
    fs::write(&probe_file, PROBE).ok()?;
    Command::new(rustc)
        .arg("--edition=2018")
        .arg("--crate-name=oxidize_probe")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("--out-dir")
        .arg(out_dir)
        .arg(probe_file)
        .status()
        .ok()
}

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
        use winres::VersionInfo::*;

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
        .args(&["rev-parse", "--short", "HEAD"])
        .output()?;

    let rev = std::str::from_utf8(&output.stdout)?.trim();

    if let Ok(oxidize_version) = env::var("OXIDIZE_VERSION") {
        version = oxidize_version;
        user_agent = format!(
            "OxidizeBot/{} (git {rev}; +{url})",
            version,
            rev = rev,
            url = URL
        );
    } else {
        version = format!("git-{}", rev);
        user_agent = format!("OxidizeBot/0 (git {rev}; +{url})", rev = rev, url = URL);
    }

    fs::write(out_dir.join("version.txt"), &version).context("writing version.txt")?;
    fs::write(out_dir.join("user_agent.txt"), &user_agent).context("writing user_agent.txt")?;

    // backtrace compile probe
    match backtrace_compile_probe() {
        Some(status) if status.success() => println!("cargo:rustc-cfg=backtrace"),
        _ => {}
    }

    Ok(())
}
