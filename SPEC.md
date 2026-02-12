# dpkg Specification

A declarative package manager for Arch Linux with AUR support via `yay`.

---

## Overview

`dpkg` synchronizes the system package state with a declarative configuration file. It ensures only packages explicitly declared are installed, removes orphaned packages, and supports conditional installation based on hostname.

**Key Features:**
- Declarative package management via config file
- Hostname-specific package sets (for different machines)
- AUR support through `yay` only
- Automatic orphan removal
- Paragraph-based configuration format (sort-friendly)

---

## Installation

### Prerequisites

- Arch Linux or Arch-based distribution
- `pacman` (system package manager)
- `yay` (for AUR packages - required if config contains AUR packages)

### Building from Source

```bash
git clone <repository>
cd dpkg
cargo build --release
# Binary will be at target/release/dpkg
```

---

## Configuration File Format

**Default Location:** `$HOME/.config/dpkg/pkg.conf`

**Custom Location:** Specified via `--config` flag or `DPKG_CONFIG` environment variable

### File Syntax

The configuration file uses a **paragraph-based** format. Each section is a paragraph starting with a section header.

```conf
## * // Common packages for all machines
base // essential system packages
base-devel
linux
linux-firmware
git // version control

## @LomzemDesktop // Gaming desktop setup
nvidia // proprietary GPU drivers
nvidia-utils
steam // gaming platform

## @LomzemLaptop // ThinkPad T480
xf86-video-intel // integrated graphics
tlp // battery optimization

## * // Back to common packages
firefox
docker
aur:visual-studio-code-bin
```

### Section Headers

Section headers define which hostnames the packages apply to:

| Header | Meaning |
|--------|---------|
| `## *` | Packages for **all** hostnames |
| `## @<hostname>` | Packages for **specific** hostname only |

**Important:** The space after `##` is required. `##*` is not valid.

### Package Lines

- One package per line
- Empty lines are ignored
- Leading/trailing whitespace is stripped
- Lines starting with `//` are comments (everything after `//` is ignored)
- `//` can appear inline after content to add comments
- Lines starting with `##` (double hash) are section headers

**Official Repository Packages:**
```conf
firefox
base-devel
linux-firmware
```

**AUR Packages:**
Prefix with `aur:`
```conf
aur:yay
aur:visual-studio-code-bin
aur:google-chrome
```

### Comments

Comments use C-style `//` syntax. Everything from `//` to the end of the line is ignored.

**Rules:**
- `//` starts a comment that extends to the end of the line
- Can appear on section headers, package lines, or standalone lines
- Multiple `//` on a line: the first `//` starts the comment, subsequent `//` are part of the comment
- No block comments (`/* */` is not supported)

**Examples:**

```conf
// This is a standalone comment
## * // This section applies to all hosts
base // essential system package
git // version control system
aur:visual-studio-code-bin // better than official repo version

## @LomzemDesktop
nvidia // uses // in path comments are fine // this is still a comment
```

**Note:** Comments are stripped during parsing and do not affect package names or section headers.

### Hostname Matching

- Matching is **case-sensitive exact string match**
- Use the output of the `hostname` command
- Multiple `## @<hostname>` sections with the same hostname are **merged** (order preserved)
- Hostnames can contain alphanumeric characters and hyphens
- `*` is reserved and cannot be used as a hostname

### Merge Behavior

Sections with the same header are merged together in the order they appear:

```conf
## @LomzemDesktop
nvidia

## *
firefox

## @LomzemDesktop
steam
```

For hostname `LomzemDesktop`, this installs: `nvidia`, `firefox`, `steam`

### Sorting

The format is designed to be **paragraph-sortable**. In Neovim, you can use `vip :sort` to sort paragraphs. This sorts entire sections together while maintaining internal order.

Before sort:
```conf
## @LomzemDesktop
steam
nvidia

## *
base
```

After sort:
```conf
## *
base

## @LomzemDesktop
steam
nvidia
```

---

## CLI Interface

### Usage

```
dpkg [OPTIONS] [COMMAND]
```

### Global Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--config <PATH>` | `-c` | Path to config file | `$HOME/.config/dpkg/pkg.conf` |
| `--dry-run` | `-n` | Show what would be done without executing | `false` |
| `--verbose` | `-v` | Enable verbose output | `false` |
| `--quiet` | `-q` | Suppress non-error output | `false` |
| `--help` | `-h` | Print help information | - |
| `--version` | `-V` | Print version information | - |

### Commands

#### `sync` (Default Command)

Synchronize system state with configuration file.

```bash
dpkg sync
dpkg                    # sync is the default command
dpkg -n                 # dry run - preview changes
dpkg --no-confirm       # skip removal confirmation
dpkg -c /path/to/config # use custom config file
```

**Workflow:**

1. **Parse Configuration**
   - Read config file
   - Get current hostname
   - Collect packages:
     - All packages from `## *` sections
     - All packages from `## @<current-hostname>` sections
   - Separate into official and AUR package lists

2. **Mark Packages**
   - Mark ALL currently installed packages as dependencies:
     ```bash
     sudo pacman -D --asdeps $(pacman -Qqe)
     ```
   - Mark desired packages as explicitly installed:
     ```bash
     sudo pacman -D --asexplicit <official-packages> <aur-packages>
     ```

3. **Remove Orphans**
   - Find orphaned packages (true orphans — dependencies not required by anything):
     ```bash
     pacman -Qqdt
     ```
   - Display list of packages to be removed
   - Prompt for confirmation (unless `--no-confirm`)
   - Remove orphans:
     ```bash
     pacman -Qqdt | sudo pacman -Rns -
     ```

4. **Install Missing Packages**
   - Calculate missing packages (desired - installed)
   - Install official packages:
     ```bash
     sudo pacman -S --needed --noconfirm <packages>
     ```
   - Install AUR packages:
     ```bash
     yay -S --needed --noconfirm <packages>
     ```

**Sync Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--no-confirm` | Skip confirmation for removals | `false` |
| `--only-install` | Only install missing, don't remove orphans | `false` |
| `--only-remove` | Only remove orphans, don't install | `false` |

**Exit Codes:**
- `0` - Success
- `1` - Configuration error (syntax error, file not found)
- `2` - Permission denied (need sudo for modifications)
- `3` - Package installation failed
- `4` - `yay` not found (config has AUR packages)
- `5` - Network error
- `6` - User cancelled operation

#### `status`

Display current synchronization status.

```bash
dpkg status
dpkg status -c /path/to/config
```

**Output Format:**
```
Configuration: /home/user/.config/dpkg/pkg.conf
Hostname: LomzemDesktop

Package Summary:
  Common packages (## *): 15
  Host-specific (## @LomzemDesktop): 8
  Total configured: 23
  
  Installed (official): 20
  Installed (AUR): 2
  
  Missing: 3
    - steam
    - aur:discord
    - docker
  
  Orphans: 2
    - orphan-package-1
    - orphan-package-2

Sections in config:
  ## * (15 packages)
  ## @LomzemDesktop (8 packages)
  ## @LomzemLaptop (6 packages - not current host)
```

#### `validate`

Validate configuration file syntax without applying changes.

```bash
dpkg validate
dpkg validate -c /path/to/config
```

**Validation Checks:**
- Config file exists and is readable
- Section headers are valid (`## *` or `## @<hostname>`)
- Comments are properly formatted (`//` syntax)
- No empty package names
- AUR packages have valid format (`aur:<name>`)
- No duplicate packages within the same section scope
- Hostname contains only valid characters

**Exit Codes:**
- `0` - Configuration is valid
- `1` - Syntax error (error message includes line number)
- `2` - File not found or unreadable

#### `diff`

Show differences between config and system state.

```bash
dpkg diff
```

**Output Format:**
```diff
+ steam                    // @LomzemDesktop - not installed
+ aur:discord              // @LomzemDesktop - not installed (AUR)
+ docker                   // ## * - not installed
- orphan-1                 // not in config, would be removed
- orphan-2                 // not in config, would be removed
```

Symbols:
- `+` - Package in config but not installed (would be installed)
- `-` - Package installed but not in config (would be removed as orphan)
- `!` - Package source mismatch (e.g., installed from AUR but config wants official)

---

## Core Algorithm

### Package Collection

```rust
struct Config {
    sections: Vec<Section>,
}

struct Section {
    header: Header,  // All or Hostname(String)
    packages: Vec<Package>,
}

enum Header {
    All,              // ## *
    Hostname(String), // ## @<hostname>
}

struct Package {
    name: String,
    source: PackageSource,
}

enum PackageSource {
    Official,
    Aur,
}

fn collect_packages(config: &Config, hostname: &str) -> (Vec<String>, Vec<String>) {
    let mut official = Vec::new();
    let mut aur = Vec::new();
    
    for section in &config.sections {
        let should_include = match &section.header {
            Header::All => true,
            Header::Hostname(h) => h == hostname,
        };
        
        if should_include {
            for package in &section.packages {
                match package.source {
                    PackageSource::Official => official.push(package.name.clone()),
                    PackageSource::Aur => aur.push(package.name.clone()),
                }
            }
        }
    }
    
    // Remove duplicates (keep first occurrence)
    official = remove_duplicates(official);
    aur = remove_duplicates(aur);
    
    (official, aur)
}
```

### Orphan Detection

An orphan is a package that meets ALL of the following criteria:
1. Is currently installed (`pacman -Qq`)
2. Is marked as a dependency (`pacman -Qqd`)
3. Is not required by any other installed package (`-t` flag)
4. Is NOT in the desired package list (from config)

Command to list true orphans: `pacman -Qqdt`

**Important:** The `-t` (unrequired) flag is critical. `pacman -Qqd` lists ALL dependency-installed packages, while `pacman -Qqdt` lists only those that are not required by any other installed package. Using `-Qqd` without `-t` after marking all packages as dependencies would return nearly every package on the system.

**Important:** The algorithm ONLY removes packages that pacman reports as orphans. It never removes explicitly installed packages or packages required by other packages.

### Sync Steps

```rust
fn sync(config_path: PathBuf, options: SyncOptions) -> Result<(), ExitCode> {
    // 1. Parse configuration
    let config = parse_config(&config_path)?;
    
    // 2. Get current system state
    let hostname = gethostname()?;
    let (desired_official, desired_aur) = collect_packages(&config, &hostname);
    
    // Check for AUR packages and yay availability
    if !desired_aur.is_empty() {
        check_yay_installed()?;
    }
    
    // 3. Calculate differences
    let installed_official = get_installed_official()?;
    let installed_aur = get_installed_aur()?;
    
    let to_install_official: Vec<_> = desired_official
        .iter()
        .filter(|p| !installed_official.contains(p))
        .cloned()
        .collect();
    
    let to_install_aur: Vec<_> = desired_aur
        .iter()
        .filter(|p| !installed_aur.contains(p))
        .cloned()
        .collect();
    
    let orphans = get_orphans()?;
    let unwanted_orphans: Vec<_> = orphans
        .iter()
        .filter(|p| !desired_official.contains(p) && !desired_aur.contains(p))
        .cloned()
        .collect();
    
    // 4. Dry run - just print and exit
    if options.dry_run {
        print_plan(&to_install_official, &to_install_aur, &unwanted_orphans);
        return Ok(());
    }
    
    // 5. Execute changes
    
    // Mark all as dependencies
    if !options.only_install {
        mark_all_as_deps()?;
    }
    
    // Mark desired as explicit
    if !options.only_install {
        mark_as_explicit(&desired_official)?;
        mark_as_explicit(&desired_aur)?;
    }
    
    // Remove orphans
    if !options.only_install && !unwanted_orphans.is_empty() {
        if options.no_confirm || confirm_removal(&unwanted_orphans) {
            remove_orphans()?;
        } else {
            return Err(ExitCode::UserCancelled);
        }
    }
    
    // Install missing packages
    if !options.only_remove {
        if !to_install_official.is_empty() {
            install_official(&to_install_official)?;
        }
        if !to_install_aur.is_empty() {
            install_aur(&to_install_aur)?;
        }
    }
    
    Ok(())
}
```

---

## Error Handling

### Error Categories

1. **Configuration Errors** (Exit Code 1)
   - File not found at default or specified path
   - Permission denied reading config file
   - Syntax errors (invalid section headers, malformed AUR package names)
   - Empty config file

2. **Permission Errors** (Exit Code 2)
   - User not in sudoers file
   - User cancelled sudo password prompt
   - File system permissions preventing package operations

3. **Installation Errors** (Exit Code 3)
   - Package not found in repositories
   - Package conflicts
   - Broken dependencies
   - Signature verification failures

4. **AUR Errors** (Exit Code 4)
   - `yay` binary not found in PATH
   - AUR package not found
   - AUR build failures

5. **Network Errors** (Exit Code 5)
   - Cannot reach package repositories
   - Cannot reach AUR
   - Timeout during package download

6. **User Cancellation** (Exit Code 6)
   - User declined removal confirmation

### Error Messages

All error messages should be clear and actionable:

```
Error: Configuration file not found
  Path: /home/user/.config/dpkg/pkg.conf
  Hint: Create the file or specify a different path with --config

Error: Invalid section header at line 15
  Content: ##desktop
  Expected: ## * or ## @<hostname>
  Hint: Section headers must have a space after ##

Error: AUR packages found but yay is not installed
  AUR packages in config: visual-studio-code-bin, discord
  Hint: Install yay: git clone https://aur.archlinux.org/yay.git && cd yay && makepkg -si

Error: Package 'nvidia' not found in repositories
  Hint: Check the package name or install from AUR with 'aur:nvidia'
```

### Recovery Suggestions

When operations fail, provide recovery hints:

- If sync partially fails, suggest running with `--only-install` or `--only-remove`
- If permission denied, remind about sudo
- If package not found, suggest checking AUR
- If config has errors, suggest using `dpkg validate`

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DPKG_CONFIG` | Path to configuration file | `$HOME/.config/dpkg/pkg.conf` |
| `DPKG_NO_COLOR` | Disable colored output | not set |
| `PACMAN` | Path to pacman binary | `pacman` |
| `YAY` | Path to yay binary | `yay` |

---

## Example Configuration Files

### Minimal Example

```conf
## * // System essentials
base
base-devel
linux
linux-firmware
grub
git

## @LomzemDesktop // Desktop with NVIDIA GPU
nvidia
nvidia-utils

## @LomzemLaptop // Laptop with Intel graphics
xf86-video-intel
```

### Full Featured Example

```conf
// System essentials - installed on all machines
## *
base
base-devel
linux
linux-firmware
grub
efibootmgr
intel-ucode

// Bootloader configuration
## @LomzemDesktop
efitools

## @LomzemLaptop
efitools
bolt

// Essential tools
## *
git
git-lfs
curl
wget
htop
neovim
vim
tmux
fzf
ripgrep
fd
exa
bat
dust
procs
tree

// Development environment
## *
docker
docker-compose
nodejs
npm
python
python-pip
rustup
cargo

## @LomzemDesktop
aur:visual-studio-code-bin
aur:intellij-idea-ultimate-edition

## @LomzemLaptop
neovim // Use terminal editor on laptop

// Desktop environment
## @LomzemDesktop
sway
waybar
mako
wofi
wl-clipboard
grim
slurp
swayidle
swaylock

## @LomzemLaptop
sway
waybar
mako
wofi
wl-clipboard
grim
slurp
brightnessctl
tlp
tlp-rdw

// Graphics drivers
## @LomzemDesktop
nvidia
nvidia-utils
nvidia-settings

## @LomzemLaptop
xf86-video-intel
mesa
vulkan-intel

// Applications
## *
firefox
thunderbird

## @LomzemDesktop
steam
gamescope
aur:discord

## @LomzemLaptop
aur:microsoft-edge-stable-bin

// Fonts
## *
noto-fonts
noto-fonts-cjk
noto-fonts-emoji
ttf-dejavu
ttf-liberation

## @LomzemDesktop
aur:nerd-fonts-complete

// Shell and terminal
## *
zsh
zsh-completions
zsh-syntax-highlighting
zsh-autosuggestions
alacritty

// File management
## *
nnn
ranger
ncdu
gdu

// Media
## *
pipewire
pipewire-pulse
pipewire-alsa
wireplumber
pamixer

## @LomzemDesktop
spotify-launcher
mpv
imv
```

---

## Safety Features

### Before First Run

On the first execution, display a warning:

```
⚠️  First time setup detected

Before proceeding, it's recommended to back up your currently installed packages:

    pacman -Qqe > ~/pkglist-backup.txt

This will allow you to restore your system if needed.

Continue? [y/N]
```

### Dry Run Mode

The `-n, --dry-run` flag shows exactly what would happen without making changes:

```
$ dpkg sync -n
Configuration: /home/user/.config/dpkg/pkg.conf
Hostname: LomzemDesktop

Would install (official):
  steam
  docker
  gamescope

Would install (AUR):
  visual-studio-code-bin
  discord

Would remove (orphans):
  orphan-1
  orphan-2
  unused-dep-3

No changes made (dry run)
```

### Removal Confirmation

Unless `--no-confirm` is specified, removals require user confirmation:

```
The following packages will be removed (orphans):
  orphan-1
  orphan-2
  unused-dep-3

Proceed with removal? [y/N]:
```

### Protected Packages

The tool relies on pacman's orphan detection which automatically protects:
- Explicitly installed packages
- Packages required by other installed packages
- Base system packages (marked as dependencies but required)

### AUR Package Verification

Before attempting to install AUR packages:
1. Verify `yay` is installed and accessible
2. Verify `yay` can connect to AUR
3. Fail early if AUR is unreachable (before modifying system state)

---

## Implementation Notes

### Dependencies

**Runtime:**
- `pacman` - System package manager
- `yay` - AUR helper (only required if config uses AUR packages)

**Build:**
- Rust toolchain
- `clap` - Command line argument parsing
- `hostname` crate - Get system hostname

### External Commands

The implementation shells out to these commands:

| Command | Purpose | Example |
|---------|---------|---------|
| `hostname` | Get current hostname | `hostname` |
| `pacman -Qqe` | List explicitly installed packages | `pacman -Qqe` |
| `pacman -Qqdt` | List true orphan packages | `pacman -Qqdt` |
| `pacman -D --asdeps` | Mark packages as dependencies | `pacman -D --asdeps pkg1 pkg2` |
| `pacman -D --asexplicit` | Mark packages as explicit | `pacman -D --asexplicit pkg1 pkg2` |
| `pacman -Rns` | Remove orphans and their configs | `pacman -Rns -` |
| `pacman -S` | Install packages | `pacman -S --needed --noconfirm pkg1 pkg2` |
| `yay -S` | Install AUR packages | `yay -S --needed --noconfirm pkg1 pkg2` |

### Error Output

All error messages go to stderr. Normal output goes to stdout (unless `--quiet`).

### Colored Output

Use colors for better readability (respect `DPKG_NO_COLOR`):
- Green: Success, installed packages
- Red: Errors, removed packages
- Yellow: Warnings, orphans
- Blue: Info, headers
- Cyan: Dry run indicators

---

## Testing Strategy

### Unit Tests

- Config parsing (valid and invalid inputs)
- Package collection logic
- Hostname filtering
- Duplicate removal

### Integration Tests

- Test with mock pacman/yay scripts
- Test various config file scenarios
- Test error conditions

### Manual Testing

- Test on clean Arch installation
- Test with various hostname configurations
- Test orphan removal scenarios
- Test AUR package installations

---

## Future Considerations

These features are explicitly **out of scope** for the initial implementation but may be added later:

- Support for multiple AUR helpers (paru, aura, etc.)
- `dpkg add <package>` command to modify config file
- Package version constraints (e.g., `firefox>=120`)
- `dpkg export` to generate config from current system
- Include directives for multiple config files
- Hook system (pre/post sync scripts)
- Service management (enable/disable systemd units)
- File tracking (dotfiles management)
- Package groups and meta-packages

---

## Changelog

### v1.0.0 (Initial Release)

- Basic sync functionality
- Hostname-specific package sets
- AUR support via yay
- Orphan removal with confirmation
- Dry-run mode
- Config validation
- Status and diff commands

---

## License

[Specify your license here]

---

## Contributing

[Contribution guidelines if applicable]

---

## FAQ

**Q: Why not just use Ansible/Puppet/Chef?**
A: This tool is intentionally lightweight and purpose-built for personal Arch Linux systems. It doesn't require a server, agent, or complex configuration.

**Q: Can I use paru instead of yay?**
A: Not in this version. The implementation uses yay-specific features. Support for other AUR helpers is a future consideration.

**Q: What happens if I specify a package that exists in both official repos and AUR?**
A: The tool prioritizes official repositories. Only prefix with `aur:` if you specifically want the AUR version.

**Q: Will this remove packages I installed manually?**
A: Only if they're orphaned (not required by other packages and marked as dependencies). Explicitly installed packages are protected.

**Q: Can I use this on multiple machines with different configs?**
A: Yes! The hostname feature (`## @hostname`) is designed exactly for this. Use `## *` for common packages and `## @<hostname>` for machine-specific ones.

**Q: How do I see what would happen before applying changes?**
A: Use `dpkg sync -n` or `dpkg diff` for a dry run.

**Q: I accidentally removed packages. How do I restore them?**
A: If you created the backup as suggested on first run:
```bash
pacman -S --needed - < ~/pkglist-backup.txt
```

---

## Support

For bug reports and feature requests, please open an issue at [repository URL].

For questions and discussions, use [discussion forum/Discord/etc].
