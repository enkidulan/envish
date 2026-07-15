# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-14

### Added

- Load named TOML environment profiles into a nested shell
- `edit`, `list`, and `path` commands
- Clear CLI errors instead of panics
- MIT license, README, pre-commit hooks, and GitHub Actions CI
- Binary releases on version tags (curl install from GitHub Releases)

### Fixed

- String profile values no longer keep TOML quotes when set in the environment
