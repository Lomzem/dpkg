use std::collections::HashSet;
use std::path::Path;

use crate::config::{collect_packages, parse_config, Header, PackageSource};
use crate::error::DpkgError;
use crate::output;
use crate::system;

pub fn run(config_path: &Path, quiet: bool) -> Result<(), DpkgError> {
    let config = parse_config(config_path)?;
    let hostname = system::get_hostname()?;
    let (desired_official, desired_aur) = collect_packages(&config, &hostname);

    let installed = system::get_explicitly_installed()?;
    let installed_set: HashSet<&str> = installed.iter().map(|s| s.as_str()).collect();

    if quiet {
        return Ok(());
    }

    output::info(&format!("Configuration: {}", config_path.display()));
    output::info(&format!("Hostname: {hostname}"));
    println!();

    // Count packages per section type
    let common_count: usize = config
        .sections
        .iter()
        .filter(|s| s.header == Header::All)
        .map(|s| s.packages.len())
        .sum();

    let host_count: usize = config
        .sections
        .iter()
        .filter(|s| s.header == Header::Hostname(hostname.clone()))
        .map(|s| s.packages.len())
        .sum();

    let total = desired_official.len() + desired_aur.len();

    output::plain("Package Summary:");
    output::plain(&format!("  Common packages (## *): {common_count}"));
    output::plain(&format!("  Host-specific (## @{hostname}): {host_count}"));
    output::plain(&format!("  Total configured: {total}"));
    println!();

    // Count installed by type
    let installed_official_count = desired_official
        .iter()
        .filter(|p| installed_set.contains(p.as_str()))
        .count();
    let installed_aur_count = desired_aur
        .iter()
        .filter(|p| installed_set.contains(p.as_str()))
        .count();

    output::plain(&format!("  Installed (official): {installed_official_count}"));
    output::plain(&format!("  Installed (AUR): {installed_aur_count}"));
    println!();

    // Missing packages
    let missing_official: Vec<&String> = desired_official
        .iter()
        .filter(|p| !installed_set.contains(p.as_str()))
        .collect();
    let missing_aur: Vec<&String> = desired_aur
        .iter()
        .filter(|p| !installed_set.contains(p.as_str()))
        .collect();
    let missing_count = missing_official.len() + missing_aur.len();

    output::plain(&format!("  Missing: {missing_count}"));
    for pkg in &missing_official {
        output::plain(&format!("    - {pkg}"));
    }
    for pkg in &missing_aur {
        output::plain(&format!("    - aur:{pkg}"));
    }

    // Orphans
    let orphans = system::get_orphans()?;
    let mut all_desired: HashSet<&str> = HashSet::new();
    for p in &desired_official {
        all_desired.insert(p.as_str());
    }
    for p in &desired_aur {
        all_desired.insert(p.as_str());
    }
    let unwanted_orphans: Vec<&String> = orphans
        .iter()
        .filter(|p| !all_desired.contains(p.as_str()))
        .collect();

    println!();
    output::plain(&format!("  Orphans: {}", unwanted_orphans.len()));
    for pkg in &unwanted_orphans {
        output::plain(&format!("    - {pkg}"));
    }

    // Section listing
    println!();
    output::plain("Sections in config:");
    for section in &config.sections {
        let pkg_count = section.packages.len();
        let is_current = match &section.header {
            Header::All => true,
            Header::Hostname(h) => h == &hostname,
        };
        let suffix = if !is_current {
            " - not current host"
        } else {
            ""
        };
        let source_breakdown: String = {
            let official = section
                .packages
                .iter()
                .filter(|p| p.source == PackageSource::Official)
                .count();
            let aur = section
                .packages
                .iter()
                .filter(|p| p.source == PackageSource::Aur)
                .count();
            if aur > 0 {
                format!("{pkg_count} packages, {official} official + {aur} AUR{suffix}")
            } else {
                format!("{pkg_count} packages{suffix}")
            }
        };
        output::plain(&format!("  {} ({source_breakdown})", section.header));
    }

    Ok(())
}
