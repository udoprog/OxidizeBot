use std::env;
use std::env::consts;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Datelike;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::Timelike;
use chrono::Utc;
use regex::Regex;
use walkdir::WalkDir;

enum Release {
    Version(Version),
    Nightly(NaiveDateTime),
    Date(NaiveDate),
}

impl Release {
    fn file_version(&self) -> Result<String> {
        /// Calculate an MSI-safe version number.
        /// Unfortunately this enforces some unfortunate constraints on the available
        /// version range.
        ///
        /// The computed patch component must fit within 65535
        fn from_version(version: &Version) -> Result<String> {
            if version.patch > 64 {
                bail!(
                    "patch version must not be greater than 64: {}",
                    version.patch
                );
            }

            let mut last = 999;

            if let Some(pre) = version.pre {
                if pre >= 999 {
                    bail!(
                        "patch version must not be greater than 64: {}",
                        version.patch
                    );
                }

                last = pre;
            }

            last += version.patch * 1000;
            Ok(format!("{}.{}.{}", version.major, version.minor, last))
        }

        fn from_date_time(date_time: &NaiveDateTime) -> Result<String> {
            let date = date_time.date();

            Ok(format!(
                "{}.{}.{}",
                date.year() - 2023,
                date.month(),
                date.day() * 100 + date.day() + date_time.hour()
            ))
        }

        fn from_date(date: &NaiveDate) -> Result<String> {
            Ok(format!(
                "{}.{}.{}",
                date.year() - 2023,
                date.month(),
                date.day() * 100 + date.day()
            ))
        }

        match self {
            Release::Version(version) => from_version(version),
            Release::Nightly(date_time) => from_date_time(date_time),
            Release::Date(date) => from_date(date),
        }
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Release::Version(version) => version.fmt(f),
            Release::Date(date) => date.fmt(f),
            Release::Nightly(date_time) => {
                let date = date_time.date();
                write!(
                    f,
                    "nightly-{}.{}.{}.{}",
                    date.year(),
                    date.month(),
                    date.day(),
                    date_time.hour()
                )
            }
        }
    }
}

pub(crate) struct SignTool {
    root: PathBuf,
    signtool: PathBuf,
    password: String,
}

impl SignTool {
    /// Construct a new signtool signer.
    pub(crate) fn open(root: PathBuf, signtool: PathBuf, password: String) -> Option<Self> {
        if !signtool.is_file() {
            return None;
        }

        Some(Self {
            root,
            signtool,
            password,
        })
    }

    /// Sign the given path with the given description.
    fn sign(&self, path: &Path, what: &str) -> Result<()> {
        println!("Signing: {}", path.display());

        let cert = self.root.join("bot/res/cert.pfx");

        let status = Command::new(&self.signtool)
            .args(["sign", "/f"])
            .arg(&cert)
            .args([
                "/d",
                what,
                "/du",
                "https://github.com/udoprog/OxidizeBot",
                "/p",
                self.password.as_str(),
            ])
            .arg(path)
            .status()?;

        if !status.success() {
            bail!("failed to run signtool");
        }

        Ok(())
    }
}

struct WixBuilder {
    candle_bin: PathBuf,
    light_bin: PathBuf,
    wixobj_path: PathBuf,
    installer_path: PathBuf,
}

impl WixBuilder {
    /// Construct a new WIX builder.
    fn new(out: &Path, release: &Release) -> Result<Self> {
        let wix_bin = match env::var_os("WIX") {
            Some(wix_bin) => PathBuf::from(wix_bin).join("bin"),
            None => bail!("missing: WIX"),
        };

        if !wix_bin.is_dir() {
            bail!("missing: {}", wix_bin.display());
        }

        let candle_bin = wix_bin.join("candle.exe");

        if !candle_bin.is_file() {
            bail!("missing: {}", candle_bin.display());
        }

        let light_bin = wix_bin.join("light.exe");

        if !light_bin.is_file() {
            bail!("missing: {}", light_bin.display());
        }

        let base = format!(
            "oxidize-{release}-{os}-{arch}",
            os = consts::OS,
            arch = consts::ARCH
        );

        let wixobj_path = out.join(format!("{}.wixobj", base));
        let installer_path = out.join(format!("{}.msi", base));

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
            _ => "x86",
        };

        let mut command = Command::new(&self.candle_bin);

        command
            .arg(format!("-dVersion={}", file_version))
            .arg(format!("-dPlatform={}", platform))
            .args(["-ext", "WixUtilExtension"])
            .arg("-o")
            .arg(&self.wixobj_path)
            .arg(source);

        println!("running: {:?}", command);

        let status = command.status()?;

        if !status.success() {
            bail!("candle: failed: {}", status);
        }

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
            .arg(&self.installer_path);

        println!("running: {:?}", command);

        let status = command.status()?;

        if !status.success() {
            bail!("light: failed: {}", status);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Version {
    base: String,
    major: u32,
    minor: u32,
    patch: u32,
    pre: Option<u32>,
}

impl Version {
    /// Open a version by matching it against the given string.
    pub(crate) fn parse(version: impl AsRef<str>) -> Result<Version> {
        let version = version.as_ref();
        let version_re = Regex::new(r"^(\d+)\.(\d+)\.(\d+)(-.+\.(\d+))?$")?;
        let m = version_re.captures(version).context("invalid version")?;

        let major: u32 = str::parse(&m[1])?;
        let minor: u32 = str::parse(&m[2])?;
        let patch: u32 = str::parse(&m[3])?;
        let pre: Option<u32> = m.get(5).map(|s| str::parse(s.as_str())).transpose()?;

        Ok(Self {
            base: version.to_string(),
            major,
            minor,
            patch,
            pre,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.base.fmt(fmt)
    }
}

impl AsRef<[u8]> for Version {
    fn as_ref(&self) -> &[u8] {
        self.base.as_bytes()
    }
}

impl AsRef<OsStr> for Version {
    fn as_ref(&self) -> &OsStr {
        self.base.as_ref()
    }
}

fn cargo(args: &[&str]) -> Result<()> {
    println!("cargo {}", args.join(" "));
    let status = Command::new("cargo").args(args).status()?;

    if !status.success() {
        bail!("failed to run cargo");
    }

    Ok(())
}

fn create_zip(file: &Path, it: impl IntoIterator<Item = PathBuf>) -> Result<()> {
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut zip = zip::ZipWriter::new(fs::File::create(file)?);

    for p in it {
        println!("Adding to zip: {}", p.display());

        let file_name = p
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| anyhow!("file name is not a string"))?;

        zip.start_file(file_name, options)?;
        let mut from = fs::File::open(&p)?;
        io::copy(&mut from, &mut zip)?;
    }

    zip.finish()?;
    Ok(())
}

/// Copy a bunch of files with the matching file extension from one directory to another.
fn copy_files<F>(from: &Path, target: &Path, ext: &str, visitor: F) -> Result<()>
where
    F: Fn(&Path) -> Result<()>,
{
    if !target.is_dir() {
        fs::create_dir_all(target)?;
    }

    for e in WalkDir::new(from) {
        let e = e?;

        let source = e.path();

        if source.extension() != Some(OsStr::new(ext)) {
            continue;
        }

        let name = source.file_name().ok_or_else(|| anyhow!("no file name"))?;
        let target = target.join(name);
        println!("{} -> {}", source.display(), target.display());
        fs::copy(source, &target)?;
        visitor(&target)?;
    }

    Ok(())
}

/// Create a zip distribution.
fn create_zip_dist(dest: &Path, release: &Release, files: Vec<PathBuf>) -> Result<()> {
    if !dest.is_dir() {
        fs::create_dir_all(dest)?;
    }

    let zip_file = dest.join(format!(
        "oxidize-{release}-{os}-{arch}.zip",
        os = consts::OS,
        arch = consts::ARCH
    ));

    println!("Creating Zip File: {}", zip_file.display());
    create_zip(&zip_file, files)?;
    Ok(())
}

/// Perform a Windows build.
fn windows_build(root: &Path, release: Release) -> Result<()> {
    let file_version = release.file_version()?;
    env::set_var("OXIDIZE_VERSION", release.to_string());
    env::set_var("OXIDIZE_FILE_VERSION", &file_version);

    let signer = match (env::var("SIGNTOOL"), env::var("CERTIFICATE_PASSWORD")) {
        (Ok(signtool), Ok(password)) => {
            SignTool::open(root.to_owned(), PathBuf::from(signtool), password)
        }
        _ => None,
    };

    let exe = root.join("target").join("release").join("oxidize.exe");
    let wix_dir = root.join("target").join("wix");
    let upload = root.join("target").join("upload");

    let wix_builder = WixBuilder::new(&wix_dir, &release)?;

    if !exe.is_file() {
        println!("building: {}", exe.display());
        cargo(&[
            "build",
            "--manifest-path=bot/Cargo.toml",
            "--release",
            "--bin",
            "oxidize",
            "--features",
            "windows",
        ])?;
    }

    if let Some(signer) = signer.as_ref() {
        signer.sign(&exe, "OxidizeBot")?;
    }

    wix_builder.build(&root.join("wix").join("main.wxs"), &file_version)?;
    wix_builder.link()?;

    copy_files(&wix_dir, &upload, "msi", |file| {
        if let Some(signer) = &signer {
            signer.sign(file, "OxidizeBot Installer")?;
        }

        Ok(())
    })?;

    create_zip_dist(&upload, &release, vec![root.join("README.md"), exe])?;
    Ok(())
}

/// Perform a Linux build.
fn linux_build(root: &Path, release: Release) -> Result<()> {
    env::set_var("OXIDIZE_VERSION", release.to_string());
    println!("Release: {release}");

    // Install cargo-deb for building the package below.
    cargo(&["install", "cargo-deb"])?;

    let exe = root.join("target/release/oxidize");
    let debian_dir = root.join("target/debian");
    let upload = root.join("target/upload");

    if !debian_dir.is_dir() {
        cargo(&[
            "deb",
            "-p",
            "oxidize",
            "--deb-version",
            &release.to_string(),
        ])?;
    }

    copy_files(&debian_dir, &upload, "deb", |_| Ok(()))?;

    if !exe.is_file() {
        println!("building: {}", exe.display());
        cargo(&[
            "build",
            "--manifest-path=bot/Cargo.toml",
            "--release",
            "--bin",
            "oxidize",
        ])?;
    }

    create_zip_dist(&upload, &release, vec![root.join("README.md"), exe])?;
    Ok(())
}

/// Get the github release to build.
fn github_release() -> Release {
    match github_ref_version() {
        Err(error) => {
            println!("Assuming nightly release since we couldn't determine tag: {error}");
            Release::Nightly(Utc::now().naive_local())
        }
        Ok(version) => Release::Version(version),
    }
}

/// Get the version from GITHUB_REF.
fn github_ref_version() -> Result<Version> {
    let version = match env::var("GITHUB_REF") {
        Ok(version) => version,
        _ => bail!("missing: GITHUB_REF"),
    };

    let mut it = version.split('/');

    let version = match (it.next(), it.next(), it.next()) {
        (Some("refs"), Some("tags"), Some(version)) => Version::parse(version)?,
        _ => bail!("expected GITHUB_REF: refs/tags/*"),
    };

    Ok(version)
}

/// Perform a MacOS build.
fn macos_build(root: &Path, release: Release) -> Result<()> {
    env::set_var("OXIDIZE_VERSION", release.to_string());

    let exe = root.join("target/release/oxidize");
    let upload = root.join("target/upload");

    if !exe.is_file() {
        println!("building: {}", exe.display());
        cargo(&[
            "build",
            "--manifest-path=bot/Cargo.toml",
            "--release",
            "--bin",
            "oxidize",
        ])?;
    }

    create_zip_dist(&upload, &release, vec![root.join("README.md"), exe])?;
    Ok(())
}

fn main() -> Result<()> {
    let root = env::current_dir()?;
    println!("root: {}", root.display());

    let mut it = std::env::args().skip(1);
    let mut release = None;

    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--channel" => {
                let channel = it.next().context("missing --channel argument")?;

                release = match (channel.as_str(), NaiveDate::from_str(channel.as_str())) {
                    (_, Ok(date)) => Some(Release::Date(date)),
                    ("nightly", _) => Some(Release::Nightly(Utc::now().naive_utc())),
                    _ => None,
                };
            }
            "--version" => {
                let version = it.next().context("missing --version argument")?;
                release = Some(Release::Version(Version::parse(version.as_str())?));
            }
            _ => {
                bail!("unsupported `{arg}`");
            }
        }
    }

    let release = release.unwrap_or_else(github_release);
    println!("Release: {}", release);

    if cfg!(target_os = "windows") {
        windows_build(&root, release)?;
    } else if cfg!(target_os = "linux") {
        linux_build(&root, release)?;
    } else if cfg!(target_os = "macos") {
        macos_build(&root, release)?;
    } else {
        bail!("Unsupported operating system: {}", consts::OS);
    }

    Ok(())
}
