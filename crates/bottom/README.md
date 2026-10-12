bottom project forked from https://github.com/ClementTsang/bottom

Vendored from upstream commit `c9ffc41e51de881cc5935abff73e255ff0bbb59d` (2025-01-07):
https://github.com/ClementTsang/bottom/commit/c9ffc41e51de881cc5935abff73e255ff0bbb59d

At killa commit `2dc2223` ("vendor bottom"), all 139 files under `src/`
plus `Cargo.toml` and `Cargo.lock` matched that upstream commit blob-for-blob. Only `build.rs`, `LICENSE`, and `README.md` were
not vendored. Local changes on top are in `Cargo.toml`, `src/lib.rs`,
`src/utils/cancellation_token.rs`, `src/data_collection/processes.rs`,
and `src/data_collection/processes/linux.rs` (added Linux-only
`ProcessHarvest.cpu_time` with cumulative user + system CPU time).
