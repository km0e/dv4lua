# Version 0.1.0 (2025-07-27)

This is the first alpha release of the project. It includes the following features:

- Synchronization of files between local and remote directories.
- Execution of commands on local and remote machines.
- Package management for installing packages.
- Basic dotfile management for managing dotfiles.

# Version 0.1.1 (2025-08-06)

This release includes the following improvements and bug fixes:

- Inserted detected os into the config.
- Better package management.

# Version 0.1.3 (2025-08-11)

This release includes the following improvements and bug fixes:

- Added support for using network download configuration files protocol.
- Renamed `op.copy` to `op.sync`.
- Dotfile management improvements.

# Version 0.1.4 (2025-09-01)

This release includes the following improvements:

- Added support for SSH ProxyJump.
- Improved Execution of commands.

# Version 0.1.5 (2025-09-03)

- Fixed a bug in the cache system that caused incorrect behavior when the cache file was missing or corrupted.
- Migrate single-user API to UM.
- Add `write` and `read` operations.

# Version 0.1.6 (2025-09-05)

## Breaking Changes

- `dot:add_scheme`: Removed `__network__` scheme support. Use `dv:dl` and `dot:add_scheme("cur", ... )` instead.
- `dv.op`: Removed and replaced with `dv:sync`.
- `um`, `pm`, `dot`: Get instances via `dv:um()`, `dv:pm()`, and `dv:dot()`.

## Bug Fixes

- `Dot`: `sync` must actually copy files.

## Improvements

- `dv:dl`: Support downloading files from URLs to local paths.
- `dv:json`: Added `dv:json.encode` and `dv:json.decode` for JSON handling.
