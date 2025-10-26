# Security Advisory Exceptions

This file documents known security advisories that have been reviewed and determined to be acceptable for this project.

## RUSTSEC-2025-0111: tokio-tar PAX header vulnerability

- **Affected version**: tokio-tar 0.3.1
- **Introduced via**: testcontainers 0.25.0 → testcontainers-modules 0.13.0
- **Impact**: File smuggling vulnerability in tar extraction
- **Justification**: This dependency is only used in integration tests with testcontainers, which is a test-only dependency. The tar extraction happens in the container, not in production code. Waiting for upstream testcontainers to update to a fixed version.
- **Status**: ALLOWED (test-only dependency, awaiting upstream fix)
- **CI Status**: Ignored via `deny.toml` using cargo-deny

### To upgrade when available

Update testcontainers and testcontainers-modules to versions that depend on fixed tokio-tar.

### Security Checks

All security checks pass with:

```bash
cargo deny check advisories
```

The vulnerability is explicitly ignored in the workspace `deny.toml` file with reasoning.
