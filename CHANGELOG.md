# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/0hirume/breathers/compare/v0.5.9...v0.6.0) - 2026-10-11

### Added

- [**breaking**] format C and C++ with Tree-sitter
- [**breaking**] remove language server support
- add ergonomic CLI modes and short aliases

### Fixed

- preserve LF checkout for test fixtures
- *(ci)* match Mise installer tool identities

### Other

- normalize repository text files to LF
- automate versioning and binary releases
- streamline platform checks with Nushell
- share statement unwrapping
- simplify README wording
- parallelize batch formatting with Rayon
- tighten Rust and Clippy lints
- update dependencies and enable default features
- extract multiline examples into frozen fixtures
- consolidate formatter traversal and test ownership
