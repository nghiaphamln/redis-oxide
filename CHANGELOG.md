# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of redis-oxide
- Automatic topology detection (Standalone vs Cluster)
- MOVED and ASK redirect handling for Redis Cluster
- Multiplexed connection strategy
- Connection pool strategy
- Type-safe command builders
- Basic string operations: GET, SET, DEL, EXISTS, EXPIRE, TTL, INCR, DECR
- Comprehensive error handling
- Full async/await support with Tokio
- Unit tests and integration tests
- Examples and documentation

## [0.1.0] - 2025-10-22

### Added
- Initial release
