use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub signature: String,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("network")]
    Network,
    #[error("manifest")]
    Manifest,
    #[error("no_windows_asset")]
    NoWindowsAsset,
    #[error("signature_config")]
    SignatureConfig,
    #[error("signature_invalid")]
    SignatureInvalid,
    #[error("package")]
    Package,
    #[error("install")]
    Install,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    #[serde(rename = "notes")]
    _notes: Option<String>,
    platforms: HashMap<String, PlatformAsset>,
}

#[derive(Debug, Deserialize)]
struct PlatformAsset {
    signature: String,
    url: String,
}

pub fn check(endpoint: &str) -> Result<Option<UpdateInfo>, UpdateError> {
    let client = client()?;
    let response = client
        .get(endpoint)
        .send()
        .map_err(|_| UpdateError::Network)?;
    if !response.status().is_success() {
        return Err(UpdateError::Network);
    }
    let manifest = response
        .json::<Manifest>()
        .map_err(|_| UpdateError::Manifest)?;
    let remote = parse_version(&manifest.version)?;
    let current = parse_version(env!("CARGO_PKG_VERSION"))?;
    if remote <= current {
        return Ok(None);
    }
    let asset = manifest
        .platforms
        .get("windows-x86_64")
        .ok_or(UpdateError::NoWindowsAsset)?;

    Ok(Some(UpdateInfo {
        version: manifest.version,
        url: asset.url.clone(),
        signature: asset.signature.clone(),
    }))
}

pub fn download_and_launch(update: &UpdateInfo, public_key: &str) -> Result<(), UpdateError> {
    let package = download(&update.url)?;
    verify(&package, &update.signature, public_key)?;

    let root = std::env::temp_dir()
        .join("CodexGaugeNative")
        .join(&update.version);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).map_err(|_| UpdateError::Package)?;
    let installer = materialize_installer(&package, &update.url, &root)?;
    if update.url.to_ascii_lowercase().contains(".portable.exe") {
        launch_portable_replacer(&installer)
    } else {
        launch_installer(&installer)
    }
}

pub fn run_replacement_helper() -> bool {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.get(1).and_then(|value| value.to_str()) != Some("--replace") {
        return false;
    }
    let Some(target) = args.get(2).map(PathBuf::from) else {
        return true;
    };

    let source = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return true,
    };
    for _ in 0..120 {
        if fs::copy(&source, &target).is_ok() {
            let _ = Command::new(&target).spawn();
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    true
}

fn client() -> Result<reqwest::blocking::Client, UpdateError> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| UpdateError::Network)
}

fn parse_version(value: &str) -> Result<Version, UpdateError> {
    Version::parse(value.trim_start_matches('v')).map_err(|_| UpdateError::Manifest)
}

fn download(url: &str) -> Result<Vec<u8>, UpdateError> {
    let mut response = client()?
        .get(url)
        .send()
        .map_err(|_| UpdateError::Network)?;
    if !response.status().is_success() {
        return Err(UpdateError::Network);
    }
    if response
        .content_length()
        .is_some_and(|size| size > 256 * 1024 * 1024)
    {
        return Err(UpdateError::Package);
    }
    let mut bytes = Vec::new();
    response
        .read_to_end(&mut bytes)
        .map_err(|_| UpdateError::Network)?;
    if bytes.is_empty() || bytes.len() > 256 * 1024 * 1024 {
        return Err(UpdateError::Package);
    }
    Ok(bytes)
}

fn verify(package: &[u8], signature: &str, public_key: &str) -> Result<(), UpdateError> {
    if public_key.trim().is_empty() || public_key.contains("TODO_") {
        return Err(UpdateError::SignatureConfig);
    }
    let key = if public_key.contains('\n') {
        PublicKey::decode(public_key)
    } else {
        PublicKey::from_base64(public_key.trim())
    }
    .map_err(|_| UpdateError::SignatureConfig)?;
    let signature = Signature::decode(signature).map_err(|_| UpdateError::SignatureInvalid)?;
    key.verify(package, &signature, false)
        .map_err(|_| UpdateError::SignatureInvalid)
}

fn materialize_installer(package: &[u8], url: &str, root: &Path) -> Result<PathBuf, UpdateError> {
    let lower = url.to_ascii_lowercase();
    if lower.ends_with(".zip") {
        return extract_installer(package, root);
    }

    let extension = if lower.ends_with(".msi") {
        "msi"
    } else {
        "exe"
    };
    let path = root.join(format!("update.{extension}"));
    fs::write(&path, package).map_err(|_| UpdateError::Package)?;
    Ok(path)
}

fn extract_installer(package: &[u8], root: &Path) -> Result<PathBuf, UpdateError> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(package)).map_err(|_| UpdateError::Package)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| UpdateError::Package)?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let extension = name
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension != "exe" && extension != "msi" {
            continue;
        }
        let output = root.join(format!("update.{extension}"));
        let mut file = File::create(&output).map_err(|_| UpdateError::Package)?;
        std::io::copy(&mut entry, &mut file).map_err(|_| UpdateError::Package)?;
        file.flush().map_err(|_| UpdateError::Package)?;
        return Ok(output);
    }
    Err(UpdateError::Package)
}

fn launch_installer(path: &Path) -> Result<(), UpdateError> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let is_msi = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("msi"));
    let mut command = if is_msi {
        let mut command = Command::new("msiexec.exe");
        command.args(["/i", &path.to_string_lossy(), "/passive", "/norestart"]);
        command
    } else {
        let mut command = Command::new(path);
        command.arg("/S");
        command
    };
    command
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|_| UpdateError::Install)?;
    Ok(())
}

fn launch_portable_replacer(path: &Path) -> Result<(), UpdateError> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let current = std::env::current_exe().map_err(|_| UpdateError::Install)?;
    Command::new(path)
        .arg("--replace")
        .arg(current)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|_| UpdateError::Install)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_prefixed_versions() {
        assert_eq!(parse_version("v1.2.3").unwrap(), Version::new(1, 2, 3));
    }

    #[test]
    fn rejects_missing_signing_key() {
        let error = verify(b"package", "invalid", "").unwrap_err();
        assert!(matches!(error, UpdateError::SignatureConfig));
    }
}
