use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

use crate::config::{collect_packages, parse_config};
use crate::error::DpkgError;
use crate::output;
use crate::system;

pub struct SyncOptions {
    pub dry_run: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub no_confirm: bool,
    pub only_install: bool,
    pub only_remove: bool,
}

pub fn run(config_path: &Path, options: &SyncOptions) -> Result<(), DpkgError> {
    // 1. Parse configuration
    let config = parse_config(config_path)?;
    let hostname = system::get_hostname()?;
    let (desired_official, desired_aur) = collect_packages(&config, &hostname);

    if options.verbose {
        output::info(&format!("Configuration: {}", config_path.display()));
        output::info(&format!("Hostname: {hostname}"));
        output::info(&format!(
            "Desired packages: {} official, {} AUR",
            desired_official.len(),
            desired_aur.len()
        ));
    }

    // 2. Check for yay if AUR packages needed
    if !desired_aur.is_empty() {
        system::check_yay_installed()?;
    }

    // 3. Calculate differences
    let installed = system::get_explicitly_installed()?;
    let installed_set: HashSet<&str> = installed.iter().map(|s| s.as_str()).collect();

    let to_install_official: Vec<String> = desired_official
        .iter()
        .filter(|p| !installed_set.contains(p.as_str()))
        .cloned()
        .collect();

    let to_install_aur: Vec<String> = desired_aur
        .iter()
        .filter(|p| !installed_set.contains(p.as_str()))
        .cloned()
        .collect();

    let orphans = system::get_orphans()?;
    let mut all_desired: HashSet<&str> = HashSet::new();
    for p in &desired_official {
        all_desired.insert(p.as_str());
    }
    for p in &desired_aur {
        all_desired.insert(p.as_str());
    }
    let unwanted_orphans: Vec<String> = orphans
        .into_iter()
        .filter(|p| !all_desired.contains(p.as_str()))
        .collect();

    // 4. Dry run — just print and exit
    if options.dry_run {
        print_plan(
            config_path,
            &hostname,
            &to_install_official,
            &to_install_aur,
            &unwanted_orphans,
            options.quiet,
        );
        return Ok(());
    }

    // Check if there's nothing to do
    let nothing_to_install = to_install_official.is_empty() && to_install_aur.is_empty();
    let nothing_to_remove = unwanted_orphans.is_empty();

    if nothing_to_install && nothing_to_remove {
        if !options.quiet {
            output::success("System is already in sync with configuration");
        }
        return Ok(());
    }

    // 5. Check first-run
    check_first_run()?;

    // 6. Execute changes

    // Mark all as deps → mark desired as explicit → remove orphans
    if !options.only_install {
        system::mark_all_as_deps(options.verbose)?;

        system::mark_as_explicit(&desired_official, options.verbose)?;
        system::mark_as_explicit(&desired_aur, options.verbose)?;

        if !unwanted_orphans.is_empty() {
            if !options.quiet {
                output::warning("The following packages will be removed (orphans):");
                for pkg in &unwanted_orphans {
                    output::plain(&format!("  {pkg}"));
                }
            }

            if options.no_confirm || confirm_removal()? {
                system::remove_orphans(options.verbose)?;
                if !options.quiet {
                    output::success(&format!(
                        "Removed {} orphaned packages",
                        unwanted_orphans.len()
                    ));
                }
            } else {
                return Err(DpkgError::UserCancelled);
            }
        }
    }

    // Install missing packages
    if !options.only_remove {
        if !to_install_official.is_empty() {
            if !options.quiet {
                output::info(&format!(
                    "Installing {} official packages...",
                    to_install_official.len()
                ));
            }
            system::install_official(&to_install_official, options.verbose)?;
        }

        if !to_install_aur.is_empty() {
            if !options.quiet {
                output::info(&format!(
                    "Installing {} AUR packages...",
                    to_install_aur.len()
                ));
            }
            system::install_aur(&to_install_aur, options.verbose)?;
        }
    }

    if !options.quiet {
        output::success("Sync complete");
    }

    Ok(())
}

fn print_plan(
    config_path: &Path,
    hostname: &str,
    to_install_official: &[String],
    to_install_aur: &[String],
    unwanted_orphans: &[String],
    quiet: bool,
) {
    if quiet {
        return;
    }

    output::dry_run(&format!("Configuration: {}", config_path.display()));
    output::dry_run(&format!("Hostname: {hostname}"));
    println!();

    if !to_install_official.is_empty() {
        output::dry_run("Would install (official):");
        for pkg in to_install_official {
            output::plain(&format!("  {pkg}"));
        }
        println!();
    }

    if !to_install_aur.is_empty() {
        output::dry_run("Would install (AUR):");
        for pkg in to_install_aur {
            output::plain(&format!("  {pkg}"));
        }
        println!();
    }

    if !unwanted_orphans.is_empty() {
        output::dry_run("Would remove (orphans):");
        for pkg in unwanted_orphans {
            output::plain(&format!("  {pkg}"));
        }
        println!();
    }

    if to_install_official.is_empty() && to_install_aur.is_empty() && unwanted_orphans.is_empty() {
        output::dry_run("No changes needed");
    } else {
        output::dry_run("No changes made (dry run)");
    }
}

fn confirm_removal() -> Result<bool, DpkgError> {
    println!();
    print!("Proceed with removal? [y/N]: ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|_| DpkgError::UserCancelled)?;

    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn check_first_run() -> Result<(), DpkgError> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let marker = std::path::PathBuf::from(&home).join(".config/dpkg/.initialized");

    if marker.exists() {
        return Ok(());
    }

    output::warning("First time setup detected");
    println!();
    output::plain("Before proceeding, it's recommended to back up your currently installed packages:");
    println!();
    output::plain("    pacman -Qqe > ~/pkglist-backup.txt");
    println!();
    output::plain("This will allow you to restore your system if needed.");
    println!();
    print!("Continue? [y/N]: ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|_| DpkgError::UserCancelled)?;

    let answer = input.trim().to_lowercase();
    if answer != "y" && answer != "yes" {
        return Err(DpkgError::UserCancelled);
    }

    // Create marker file
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&marker, "").ok();

    Ok(())
}
