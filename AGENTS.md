# References

- https://wiki.archlinux.org/title/Pacman
- https://wiki.archlinux.org/title/Pacman/Tips_and_tricks

# Pacman Notes

## Orphan Detection

Use `pacman -Qqdt` to list true orphans (unrequired dependency packages), NOT `pacman -Qqd`.

- `pacman -Qqd` lists ALL packages installed as dependencies — this is a huge list.
- `pacman -Qqdt` lists only dependency packages not required by any other package (true orphans).

The `-t` flag is critical. Without it, after `pacman -D --asdeps $(pacman -Qqe)`, nearly every package on the system would be listed as a "dependency" and get removed.

## Orphan Removal

Use `pacman -Rns` to remove orphans (recursive + nosave + unneeded deps), NOT `pacman -Rsu`.

Correct workflow:
```bash
pacman -Qqdt | sudo pacman -Rns -
```

## Pacman Query Flags

- `pacman -Qqe` — List explicitly installed packages only.
- `pacman -Qq` — List ALL installed packages (explicit + dependencies).
- `pacman -Qqdt` — List true orphans (deps not required by anything).

These return different sets. Know which one you need:
- Checking if a package **exists in the local DB** (i.e., is installed at all) → use `-Qq`
- Checking if a package is **explicitly installed** → use `-Qqe`

## AUR Package Marking

`pacman -D --asexplicit <pkg>` requires the package to already exist in pacman's **local** database. This means:

- **Official repo packages**: Can always be marked because they exist in the sync DB.
- **AUR packages**: Can only be marked if they've been **built and installed** (e.g., via `yay`). They do NOT exist in any pacman database until installation.

If you try to mark an uninstalled AUR package as explicit, pacman returns:
```
error: could not set install reason for package <name> (could not find or read package)
```

The fix in `sync.rs` filters `desired_aur` through `get_all_installed()` before calling `mark_as_explicit`. Uninstalled AUR packages are handled later during `install_aur`, where `yay` marks them as explicit automatically.

# Project Architecture

## Sync Command Flow (`src/commands/sync.rs`)

The sync operation runs in this order:

1. Parse config, collect desired packages for this hostname
2. Query system state (`pacman -Qqe` for explicit, `pacman -Qq` for all installed)
3. Calculate diffs (to_install, orphans)
4. If dry run, print plan and exit
5. Mark all explicitly installed as deps (`pacman -D --asdeps`)
6. Mark desired official packages as explicit (`pacman -D --asexplicit`)
7. Mark desired AUR packages as explicit — **only those already installed**
8. Remove orphans (`pacman -Rns`)
9. Install missing official packages (`pacman -S`)
10. Install missing AUR packages (`yay -S`)

## Key Files

- `src/system.rs` — All pacman/yay command wrappers. Environment vars `PACMAN` and `YAY` override binary paths.
- `src/commands/sync.rs` — Main sync logic and orchestration.
- `src/config/parser.rs` — Config file parser. Format uses `## *` (all hosts) and `## @hostname` headers, `//` comments, `aur:` prefix.
- `src/config/types.rs` — Data structures and `collect_packages()` which filters/deduplicates by hostname.
- `src/error.rs` — Error types with exit codes (1=config, 2=permission, 3=install, 4=yay missing, 6=cancelled).
