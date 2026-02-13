use std::process::Command;

use crate::error::DpkgError;

fn pacman_bin() -> String {
    std::env::var("PACMAN").unwrap_or_else(|_| "pacman".to_string())
}

fn yay_bin() -> String {
    std::env::var("YAY").unwrap_or_else(|_| "yay".to_string())
}

pub fn get_hostname() -> Result<String, DpkgError> {
    hostname::get()
        .map_err(|e| DpkgError::ConfigParse {
            line: 0,
            message: format!("Failed to get hostname: {e}"),
        })?
        .into_string()
        .map_err(|_| DpkgError::ConfigParse {
            line: 0,
            message: "Hostname contains invalid UTF-8".to_string(),
        })
}

pub fn get_explicitly_installed() -> Result<Vec<String>, DpkgError> {
    let output = Command::new(pacman_bin())
        .args(["-Qqe"])
        .output()
        .map_err(|e| DpkgError::InstallFailed(format!("Failed to run pacman: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DpkgError::InstallFailed(format!(
            "pacman -Qqe failed: {stderr}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}

pub fn get_all_installed() -> Result<Vec<String>, DpkgError> {
    let output = Command::new(pacman_bin())
        .args(["-Qq"])
        .output()
        .map_err(|e| DpkgError::InstallFailed(format!("Failed to run pacman: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DpkgError::InstallFailed(format!(
            "pacman -Qq failed: {stderr}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}

pub fn get_orphans() -> Result<Vec<String>, DpkgError> {
    let output = Command::new(pacman_bin())
        .args(["-Qqdt"])
        .output()
        .map_err(|e| DpkgError::InstallFailed(format!("Failed to run pacman: {e}")))?;

    // pacman -Qqdt returns exit code 1 when there are no orphans
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.is_empty() || output.status.code() == Some(1) {
            return Ok(Vec::new());
        }
        return Err(DpkgError::InstallFailed(format!(
            "pacman -Qqdt failed: {stderr}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}

pub fn mark_all_as_deps(verbose: bool) -> Result<(), DpkgError> {
    let explicitly = get_explicitly_installed()?;
    if explicitly.is_empty() {
        return Ok(());
    }

    if verbose {
        eprintln!("Marking {} packages as dependencies...", explicitly.len());
    }

    let output = Command::new("sudo")
        .arg(pacman_bin())
        .arg("-D")
        .arg("--asdeps")
        .args(&explicitly)
        .output()
        .map_err(|e| DpkgError::PermissionDenied(format!("Failed to run sudo pacman: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DpkgError::PermissionDenied(format!(
            "Failed to mark packages as dependencies: {}", stderr.trim()
        )));
    }

    Ok(())
}

pub fn mark_as_explicit(packages: &[String], verbose: bool) -> Result<(), DpkgError> {
    if packages.is_empty() {
        return Ok(());
    }

    if verbose {
        eprintln!(
            "Marking {} packages as explicitly installed...",
            packages.len()
        );
    }

    let output = Command::new("sudo")
        .arg(pacman_bin())
        .arg("-D")
        .arg("--asexplicit")
        .args(packages)
        .output()
        .map_err(|e| DpkgError::PermissionDenied(format!("Failed to run sudo pacman: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DpkgError::PermissionDenied(format!(
            "Failed to mark packages as explicit: {}", stderr.trim()
        )));
    }

    Ok(())
}

pub fn remove_orphans(verbose: bool) -> Result<(), DpkgError> {
    if verbose {
        eprintln!("Removing orphaned packages...");
    }

    // Get true orphans (unrequired deps) and remove them
    let orphans_output = Command::new(pacman_bin())
        .args(["-Qqdt"])
        .output()
        .map_err(|e| DpkgError::InstallFailed(format!("Failed to run pacman: {e}")))?;

    if !orphans_output.status.success() || orphans_output.stdout.is_empty() {
        return Ok(());
    }

    let orphan_list: Vec<String> = String::from_utf8_lossy(&orphans_output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    if orphan_list.is_empty() {
        return Ok(());
    }

    let output = Command::new("sudo")
        .arg(pacman_bin())
        .args(["-Rns", "--noconfirm"])
        .args(&orphan_list)
        .output()
        .map_err(|e| DpkgError::InstallFailed(format!("Failed to remove orphans: {e}")))?;

    if !output.status.success() {
        return Err(DpkgError::InstallFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(())
}

pub fn install_official(packages: &[String], verbose: bool) -> Result<(), DpkgError> {
    if packages.is_empty() {
        return Ok(());
    }

    if verbose {
        eprintln!("Installing {} official packages...", packages.len());
    }

    let output = Command::new("sudo")
        .arg(pacman_bin())
        .args(["-S", "--needed", "--noconfirm"])
        .args(packages)
        .output()
        .map_err(|e| DpkgError::InstallFailed(format!("Failed to run pacman -S: {e}")))?;

    if !output.status.success() {
        return Err(DpkgError::InstallFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(())
}

pub fn install_aur(packages: &[String], verbose: bool) -> Result<(), DpkgError> {
    if packages.is_empty() {
        return Ok(());
    }

    if verbose {
        eprintln!("Installing {} AUR packages...", packages.len());
    }

    let output = Command::new(yay_bin())
        .args(["-S", "--needed", "--noconfirm"])
        .args(packages)
        .output()
        .map_err(|e| DpkgError::InstallFailed(format!("Failed to run yay -S: {e}")))?;

    if !output.status.success() {
        return Err(DpkgError::InstallFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(())
}

pub fn check_yay_installed() -> Result<(), DpkgError> {
    let yay = yay_bin();
    let result = Command::new("which").arg(&yay).output();

    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(DpkgError::YayNotFound),
    }
}
