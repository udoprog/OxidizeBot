use anyhow::{anyhow, bail, Result};
use regex::Regex;
use std::{
    env,
    ffi::OsStr,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

#[cfg(target_os = "windows")]
pub struct SignTool {
    root: PathBuf,
    signtool: PathBuf,
    password: String,
}

#[cfg(target_os = "windows")]
impl SignTool {
    /// Construct a new signtool signer.
    pub fn open(root: PathBuf, signtool: PathBuf, password: String) -> Option<Self> {
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
            .args(&["sign", "/f"])
            .arg(&cert)
            .args(&[
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

/// Calculate an MSI-safe version number.
/// Unfortunately this enforces some unfortunate constraints on the available
/// version range.
///
/// The computed patch component must fit within 65535
#[cfg(target_os = "windows")]
fn msi_version(version: &Version) -> Result<String> {
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
    pub fn open(version: impl AsRef<str>) -> Result<Option<Version>> {
        let version_re = Regex::new(r"^(\d+)\.(\d+)\.(\d+)(-.+\.(\d+))?$")?;
        let version = version.as_ref();

        let m = match version_re.captures(version) {
            Some(m) => m,
            None => return Ok(None),
        };

        let major: u32 = str::parse(&m[1])?;
        let minor: u32 = str::parse(&m[2])?;
        let patch: u32 = str::parse(&m[3])?;
        let pre: Option<u32> = m.get(5).map(|s| str::parse(s.as_str())).transpose()?;

        Ok(Some(Self {
            base: version.to_string(),
            major,
            minor,
            patch,
            pre,
        }))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.base.fmt(fmt)
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
    let mut it = it.into_iter();

    while let Some(p) = it.next() {
        let file_name = p
            .file_name()
            .ok_or_else(|| anyhow!("file without file name"))?
            .to_str()
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
    let mut files = Vec::new();

    for e in WalkDir::new(from) {
        let e = e?;

        if e.path().extension() == Some(OsStr::new(ext)) {
            files.push(e.path().to_owned());
        }
    }

    for installer in &files {
        let name = installer
            .file_name()
            .ok_or_else(|| anyhow!("no file name"))?;

        let target = target.join(name);

        fs::copy(installer, &target)?;
        visitor(&target)?;
    }

    Ok(())
}

/// Create a zip distribution.
fn create_zip_dist(root: &Path, exe: PathBuf, version: &Version) -> Result<()> {
    create_zip(
        &root.join(format!(
            "oxidize-{version}-{os}-{arch}.zip",
            version = version,
            os = std::env::consts::OS,
            arch = std::env::consts::ARCH
        )),
        vec![root.join("README.md"), exe],
    )?;

    Ok(())
}

/// Perform a Windows build.
#[cfg(target_os = "windows")]
fn windows_build(root: &Path) -> Result<()> {
    let version = match env::var("APPVEYOR_REPO_TAG_NAME").ok() {
        Some(version) => Version::open(version)?,
        None => None,
    };

    let signer = match (env::var("SIGNTOOL"), env::var("CERTIFICATE_PASSWORD")) {
        (Ok(signtool), Ok(password)) => {
            SignTool::open(root.to_owned(), PathBuf::from(signtool), password)
        }
        _ => None,
    };

    // Is this a release?
    let version = match version {
        Some(version) => version,
        None => {
            println!("Testing...");
            cargo(&["build", "--all"])?;
            cargo(&["test", "--all"])?;
            return Ok(());
        }
    };

    let exe = root.join("target/release/oxidize.exe");

    if !exe.is_file() {
        println!("building: {}", exe.display());
        cargo(&["build", "--release", "--bin", "oxidize"])?;
    }

    if let Some(signer) = signer.as_ref() {
        signer.sign(&exe, "OxidizeBot")?;
    }

    let wix_dir = root.join("target/wix");

    if !wix_dir.is_dir() {
        let msi_version = msi_version(&version)?;

        cargo(&[
            "wix",
            "-n",
            "oxidize",
            "--install-version",
            &msi_version,
            "--nocapture",
        ])?;
    }

    copy_files(&wix_dir, &root, "msi", |file| {
        if let Some(signer) = &signer {
            signer.sign(file, "OxidizeBot Installer")?;
        }

        Ok(())
    })?;

    create_zip_dist(&root, exe, &version)?;
    Ok(())
}

/// Perform a Linux build.
#[cfg(target_os = "linux")]
fn linux_build(root: &Path) -> Result<()> {
    let version = match env::var("TRAVIS_TAG") {
        Ok(version) if version != "" => Version::open(&version)?,
        _ => None,
    };

    let pull_request = match env::var("TRAVIS_PULL_REQUEST") {
        Ok(pull_request) if pull_request != "false" => Some(str::parse::<u32>(&pull_request)?),
        _ => None,
    };

    let version = match (&pull_request, &version) {
        (None, Some(version)) => version,
        _ => {
            println!("Testing...");
            cargo(&["build", "--all"])?;
            cargo(&["test", "--all"])?;
            return Ok(());
        }
    };

    let exe = root.join("target/release/oxidize");

    let debian_dir = root.join("target/debian");

    if !debian_dir.is_dir() {
        cargo(&["deb", "-p", "oxidize"])?;
    }

    copy_files(&debian_dir, &root, "deb", |_| Ok(()))?;

    if !exe.is_file() {
        println!("building: {}", exe.display());
        cargo(&["build", "--release", "--bin", "oxidize"])?;
    }

    create_zip_dist(&root, exe, &version)?;
    Ok(())
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    println!("root: {}", root.display());

    #[cfg(target_os = "windows")]
    {
        windows_build(&root)?;
    }

    #[cfg(target_os = "linux")]
    {
        linux_build(&root)?;
    }

    Ok(())
}
