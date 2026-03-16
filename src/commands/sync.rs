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

#[derive(Debug, PartialEq)]
pub struct SyncPlan {
    pub to_install_official: Vec<String>,
    pub to_install_aur: Vec<String>,
    pub unwanted_orphans: Vec<String>,
}

/// Pure computation: given desired packages, what's installed, and current orphans,
/// determine what needs to be installed and what orphans should be removed.
pub fn compute_sync_plan(
    desired_official: &[String],
    desired_aur: &[String],
    explicitly_installed: &[String],
    orphans: Vec<String>,
) -> SyncPlan {
    let installed_set: HashSet<&str> = explicitly_installed.iter().map(|s| s.as_str()).collect();

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

    let mut all_desired: HashSet<&str> = HashSet::new();
    for p in desired_official {
        all_desired.insert(p.as_str());
    }
    for p in desired_aur {
        all_desired.insert(p.as_str());
    }
    let unwanted_orphans: Vec<String> = orphans
        .into_iter()
        .filter(|p| !all_desired.contains(p.as_str()))
        .collect();

    SyncPlan {
        to_install_official,
        to_install_aur,
        unwanted_orphans,
    }
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

    // Get all installed packages (explicit + deps) so we can filter AUR packages
    // before marking. pacman -D --asexplicit fails for packages not in the local DB,
    // and AUR packages only enter the local DB after installation via yay.
    let all_installed = system::get_all_installed()?;
    let all_installed_set: HashSet<&str> = all_installed.iter().map(|s| s.as_str()).collect();

    let orphans = system::get_orphans()?;
    let plan = compute_sync_plan(&desired_official, &desired_aur, &installed, orphans);

    let SyncPlan {
        to_install_official,
        to_install_aur,
        unwanted_orphans,
    } = &plan;

    // 4. Dry run — just print and exit
    if options.dry_run {
        print_plan(
            config_path,
            &hostname,
            to_install_official,
            to_install_aur,
            unwanted_orphans,
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

        // Only mark AUR packages that are already installed. Uninstalled AUR packages
        // aren't in pacman's local DB yet, so pacman -D --asexplicit would fail.
        // yay will mark them as explicit when it installs them.
        let installed_aur: Vec<String> = desired_aur
            .iter()
            .filter(|p| all_installed_set.contains(p.as_str()))
            .cloned()
            .collect();
        system::mark_as_explicit(&installed_aur, options.verbose)?;

        if !unwanted_orphans.is_empty() {
            if !options.quiet {
                output::warning("The following packages will be removed (orphans):");
                for pkg in unwanted_orphans {
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
            system::install_official(to_install_official, options.verbose)?;
        }

        if !to_install_aur.is_empty() {
            if !options.quiet {
                output::info(&format!(
                    "Installing {} AUR packages...",
                    to_install_aur.len()
                ));
            }
            system::install_aur(to_install_aur, options.verbose)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{collect_packages, parse_config_str};

    fn s(val: &str) -> String {
        val.to_string()
    }

    fn sv(vals: &[&str]) -> Vec<String> {
        vals.iter().map(|v| s(v)).collect()
    }

    // ── Test 1: Missing official packages are identified for installation ──

    #[test]
    fn install_missing_official_packages() {
        let installed = system::get_explicitly_installed()
            .expect("pacman -Qqe should work");
        let fake_pkg = s("zzz-nonexistent-pkg-test");
        assert!(
            !installed.contains(&fake_pkg),
            "test assumes {fake_pkg} is not installed"
        );

        let mut desired = vec![fake_pkg.clone()];
        if let Some(real) = installed.first() {
            desired.push(real.clone());
        }

        let plan = compute_sync_plan(&desired, &[], &installed, vec![]);
        assert_eq!(plan.to_install_official, vec![fake_pkg]);
    }

    // ── Test 2: Already-installed official packages are NOT reinstalled ──

    #[test]
    fn no_reinstall_already_installed_official() {
        let installed = system::get_explicitly_installed()
            .expect("pacman -Qqe should work");
        assert!(!installed.is_empty(), "system should have installed packages");

        let desired: Vec<String> = installed.iter().take(3).cloned().collect();

        let plan = compute_sync_plan(&desired, &[], &installed, vec![]);
        assert!(
            plan.to_install_official.is_empty(),
            "should not reinstall already-installed packages, got: {:?}",
            plan.to_install_official
        );
    }

    // ── Test 3: Missing AUR packages are identified for installation ──

    #[test]
    fn install_missing_aur_packages() {
        let installed = system::get_explicitly_installed()
            .expect("pacman -Qqe should work");
        let fake_aur = s("zzz-nonexistent-aur-pkg-test");
        assert!(!installed.contains(&fake_aur));

        let plan = compute_sync_plan(&[], &[fake_aur.clone()], &installed, vec![]);
        assert_eq!(plan.to_install_aur, vec![fake_aur]);
    }

    // ── Test 4: Already-installed AUR packages are NOT reinstalled ──

    #[test]
    fn no_reinstall_already_installed_aur() {
        let installed = system::get_explicitly_installed()
            .expect("pacman -Qqe should work");

        let output = std::process::Command::new("pacman")
            .args(["-Qqm"])
            .output()
            .expect("pacman -Qqm should work");

        let foreign: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.to_string())
            .collect();

        if foreign.is_empty() {
            eprintln!("SKIP: no AUR packages installed, cannot test AUR no-reinstall");
            return;
        }

        let aur_pkg = foreign
            .iter()
            .find(|p| installed.contains(p))
            .expect("at least one AUR package should be explicitly installed");

        let plan = compute_sync_plan(&[], &[aur_pkg.clone()], &installed, vec![]);
        assert!(
            plan.to_install_aur.is_empty(),
            "should not reinstall already-installed AUR package {aur_pkg}, got: {:?}",
            plan.to_install_aur
        );
    }

    // ── Test 5: Declared packages are NOT removed even if they are orphans ──

    #[test]
    fn declared_packages_not_removed() {
        let orphans = system::get_orphans()
            .expect("pacman -Qqdt should work");

        if orphans.is_empty() {
            let plan = compute_sync_plan(
                &sv(&["base", "some-orphan"]),
                &[],
                &sv(&["base", "some-orphan"]),
                vec![s("some-orphan")],
            );
            assert!(
                plan.unwanted_orphans.is_empty(),
                "declared orphan should not be removed"
            );
            return;
        }

        let orphan = orphans[0].clone();
        let plan = compute_sync_plan(
            &[orphan.clone()],
            &[],
            &sv(&[]),
            orphans.clone(),
        );
        assert!(
            !plan.unwanted_orphans.contains(&orphan),
            "orphan '{orphan}' is declared in config and should NOT be removed"
        );
        if orphans.len() > 1 {
            assert!(
                !plan.unwanted_orphans.is_empty(),
                "non-declared orphans should still be removed"
            );
        }
    }

    // ── Test 6: Hostname filtering — only matching host + ## * are included ──

    #[test]
    fn hostname_filtering() {
        let hostname = system::get_hostname()
            .expect("should get hostname");

        let config_str = format!(
            "## *\nbase\ngit\n\n## @{hostname}\nnvidia\n\n## @fake-nonexistent-host\ntlp\n"
        );
        let config = parse_config_str(&config_str)
            .expect("config should parse");

        let (official, aur) = collect_packages(&config, &hostname);
        assert!(official.contains(&s("base")), "global package 'base' should be included");
        assert!(official.contains(&s("git")), "global package 'git' should be included");
        assert!(official.contains(&s("nvidia")), "hostname package 'nvidia' should be included");
        assert!(!official.contains(&s("tlp")), "other host's package 'tlp' should NOT be included");
        assert!(aur.is_empty());
    }

    // ── Test 7: ## * (global) packages apply to any hostname ──

    #[test]
    fn global_all_hosts_behavior() {
        let config_str = "## *\nbase\ngit\nfirefox\n";
        let config = parse_config_str(config_str)
            .expect("config should parse");

        for host in &["desktop", "laptop", "server", "anything-at-all"] {
            let (official, _) = collect_packages(&config, host);
            assert_eq!(
                official,
                sv(&["base", "git", "firefox"]),
                "global packages should apply to hostname '{host}'"
            );
        }
    }

    // ── Test 8: aur: prefix correctly separates AUR from official ──

    #[test]
    fn aur_prefix_separation() {
        let config_str = "## *\nbase\ngit\naur:yay-bin\naur:paru\n";
        let config = parse_config_str(config_str)
            .expect("config should parse");

        let (official, aur) = collect_packages(&config, "anyhost");
        assert_eq!(official, sv(&["base", "git"]));
        assert_eq!(aur, sv(&["yay-bin", "paru"]));
    }
}
