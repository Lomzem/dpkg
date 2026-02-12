use std::collections::HashSet;
use std::path::Path;

use crate::config::{collect_packages, parse_config};
use crate::error::DpkgError;
use crate::output;
use crate::system;

pub fn run(config_path: &Path, quiet: bool) -> Result<(), DpkgError> {
    let config = parse_config(config_path)?;
    let hostname = system::get_hostname()?;
    let (desired_official, desired_aur) = collect_packages(&config, &hostname);

    let installed = system::get_explicitly_installed()?;
    let installed_set: HashSet<&str> = installed.iter().map(|s| s.as_str()).collect();

    let mut all_desired: HashSet<&str> = HashSet::new();
    for p in &desired_official {
        all_desired.insert(p.as_str());
    }
    for p in &desired_aur {
        all_desired.insert(p.as_str());
    }

    let mut has_diff = false;

    // Missing official packages
    for pkg in &desired_official {
        if !installed_set.contains(pkg.as_str()) {
            if !quiet {
                output::added(pkg, "// not installed");
            }
            has_diff = true;
        }
    }

    // Missing AUR packages
    for pkg in &desired_aur {
        if !installed_set.contains(pkg.as_str()) {
            if !quiet {
                output::added(&format!("aur:{pkg}"), "// not installed (AUR)");
            }
            has_diff = true;
        }
    }

    // Orphans (installed but not in config)
    let orphans = system::get_orphans()?;
    for pkg in &orphans {
        if !all_desired.contains(pkg.as_str()) {
            if !quiet {
                output::removed(pkg, "// not in config, would be removed");
            }
            has_diff = true;
        }
    }

    if !has_diff && !quiet {
        output::success("System is in sync with configuration");
    }

    Ok(())
}
