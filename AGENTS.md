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
