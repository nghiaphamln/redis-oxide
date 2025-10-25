# Dependency Compatibility Note

## Rust Edition Information

### Current Edition: Rust 2021
Redis Oxide currently uses Rust 2021 edition, which is stable and widely supported. This is the appropriate edition for production libraries.

### Rust 2024 Edition Status
Rust 2024 edition is **not yet stable** and is still under development. It is expected to be released in 2025. We do not recommend upgrading until:

- Rust 2024 edition is officially released and stable
- The ecosystem has adequate support
- All dependencies have been updated and tested
- Thorough migration testing has been performed

### Known Issue: `home` crate and `edition2024` feature

There is a known issue in the Rust ecosystem where the `home` crate (version 0.5.9+) was accidentally published with the unstable `edition2024` Cargo feature flag. This affects builds that transitively depend on this crate.

#### Affected Dependencies

The `home` crate is a transitive dependency of our project, likely coming from one of these dependencies:
- `clap`/`cargo` related tools
- `tracing`/logging infrastructure
- Development tools like `cargo-audit`

#### Workaround

The CI workflow uses a recent Rust version (1.82.0+) which should handle this properly. If you encounter the error:

```
error: failed to parse manifest at `.../home-0.5.12/Cargo.toml`
Caused by: feature `edition2024` is required
```

#### Solutions

1. **Update Rust toolchain** to the latest stable version
2. **Wait for the ecosystem** to fix the issue (the `home` crate version will likely be yanked)
3. **Pin dependencies** temporarily if needed

#### Status

This is a temporary ecosystem issue and should resolve itself once:
- The problematic `home` crate versions are yanked from crates.io
- Downstream dependencies update to non-affected versions
- The Rust toolchain fully stabilizes the referenced feature

The Redis Oxide library itself does not use the `edition2024` feature and the issue is purely a transitive dependency problem.

### Future Edition Upgrades

When Rust 2024 edition becomes stable, we will evaluate upgrading based on:
- Stability and ecosystem support
- Migration effort required
- Benefits vs. risks
- Compatibility with existing codebase
- Timeline for ecosystem adoption