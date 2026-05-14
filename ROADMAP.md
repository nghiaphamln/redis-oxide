# Roadmap

This roadmap tracks likely future work for redis-oxide. It is not a release
commitment.

## Current Release Line

The active release line is `0.2`. See `CHANGELOG.md` and crates.io for the
latest published patch version.

The current release supports:

- standalone Redis connections
- Redis Cluster routing and redirects
- connection pooling strategies
- RESP2 and RESP3 protocol support
- strings, hashes, lists, sets, sorted sets
- Pub/Sub
- Streams
- Lua scripts
- transactions and pipelines
- Sentinel configuration

## Near Term

- Expand command coverage for less common Redis commands.
- Add focused regression tests for cluster routing edge cases.
- Improve examples so each major feature has one small runnable program.
- Add CI workflows for formatting, clippy, tests, docs, audit, and deny checks.
- Review public API naming for consistency before a stable release.

## Medium Term

- Improve RESP3 coverage and examples.
- Add more connection-pool diagnostics.
- Add optional tracing spans around command execution and retries.
- Document production deployment patterns for standalone, cluster, and Sentinel.

## Long Term

- Evaluate a stable 1.0 API surface.
- Add compatibility testing across Redis versions.
- Add more benchmark scenarios for pipelines, streams, and cluster workloads.

## Release Discipline

Each release should update:

- `Cargo.toml` workspace version
- `CHANGELOG.md`
- docs that mention version-specific behavior
