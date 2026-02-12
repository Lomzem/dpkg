# dpkg

A declarative package manager for Arch Linux with AUR support via `yay`.

dpkg synchronizes the system package state with a declarative configuration file. It ensures only packages explicitly declared are installed, removes orphaned packages, and supports conditional installation based on hostname.

## Installation

Requires Arch Linux, `pacman`, and optionally `yay` (for AUR packages).

```bash
git clone <repository>
cd dpkg
cargo build --release
# Binary at target/release/dpkg
```

## Configuration

Default location: `~/.config/dpkg/pkg.conf`

```conf
// Common packages for all machines
## *
base
base-devel
linux
git
neovim
aur:visual-studio-code-bin

// Desktop-specific
## @MyDesktop
nvidia
nvidia-utils
steam

// Laptop-specific
## @MyLaptop
xf86-video-intel
tlp
brightnessctl
```

### Sections

- `## *` -- packages installed on all hosts
- `## @<hostname>` -- packages for a specific hostname only (case-sensitive match)

Multiple sections with the same header are merged in order. Duplicate packages are deduplicated (first occurrence wins).

### Packages

- One package per line
- `aur:` prefix for AUR packages (e.g., `aur:yay`)
- `//` for comments (inline or standalone)
- Empty lines and whitespace are ignored

The format is paragraph-sortable -- in Neovim, `vip :sort` sorts sections while preserving internal order.

## Usage

```bash
dpkg                     # sync (default command)
dpkg sync                # same as above
dpkg sync -n             # dry run -- preview changes
dpkg sync --no-confirm   # skip removal confirmation
dpkg sync --only-install # install missing without removing orphans
dpkg sync --only-remove  # remove orphans without installing

dpkg status              # show sync status summary
dpkg diff                # show +/- diff between config and system
dpkg validate            # check config syntax

dpkg -c /path/to/config  # use a custom config file
```

### Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--config <PATH>` | `-c` | Config file path (default: `$DPKG_CONFIG` or `~/.config/dpkg/pkg.conf`) |
| `--dry-run` | `-n` | Preview changes without executing |
| `--verbose` | `-v` | Verbose output |
| `--quiet` | `-q` | Suppress non-error output |

## How Sync Works

1. Parse config and collect packages for the current hostname
2. Mark all installed packages as dependencies (`pacman -D --asdeps`)
3. Mark desired packages as explicit (`pacman -D --asexplicit`)
4. Remove true orphans (`pacman -Qqdt | pacman -Rns`)
5. Install missing packages (`pacman -S --needed` / `yay -S --needed`)

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DPKG_CONFIG` | Config file path | `~/.config/dpkg/pkg.conf` |
| `DPKG_NO_COLOR` | Disable colored output | unset |
| `PACMAN` | pacman binary path | `pacman` |
| `YAY` | yay binary path | `yay` |

`NO_COLOR` is also respected.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Configuration error |
| 2 | Permission denied |
| 3 | Package installation failed |
| 4 | yay not found |
| 5 | Network error |
| 6 | User cancelled |
