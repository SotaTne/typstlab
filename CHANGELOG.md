# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1](https://github.com/SotaTne/typstlab/releases/tag/v0.1.1) - 2026-05-19

### Added

- *(build)* extend build system with multi-format output and MCP render tool
- *(mcp)* add stdio status server
- *(cli)* add project status command
- *(cli)* add gen paper and gen template commands
- implement LoadAction and refine creation protocol (Phase 2)
- finalize Phase 1 architecture and build implementation
- complete new architecture for build command (Phase 1)
- implement new architecture core (Phase 1)

### Changed

- *(proto)* add event envelope
- update non-install project plumbing
- *(core)* decouple stores and implement atomic staging-commit protocol
- *(core)* route build no-targets to warnings
- *(cli)* centralize project bootstrap helpers
- *(app)* preserve resolve failures and presenter fallbacks
- *(app)* model loaded project state explicitly

### Fixed

- *(clippy)* collapse nested if into match guard in BuildPresenter
- refine new command behavior and path normalization

### Other

- Codex typstlab new mcp docs ([#23](https://github.com/SotaTne/typstlab/pull/23))
- Switch release automation to release-plz
- Prepare initial release please version
- Add Apache license metadata and notice
- show docs cache in status output
- Implement JSON-based toolchain resolution
- remove legacy crates and fixtures, consolidate workspace

## [0.1.0](https://github.com/SotaTne/typstlab/releases/tag/v0.1.0) - 2026-05-18

### Added

- *(build)* extend build system with multi-format output and MCP render tool
- *(mcp)* add stdio status server
- *(cli)* add project status command
- *(cli)* add gen paper and gen template commands
- implement LoadAction and refine creation protocol (Phase 2)
- finalize Phase 1 architecture and build implementation
- complete new architecture for build command (Phase 1)
- implement new architecture core (Phase 1)

### Changed

- *(proto)* add event envelope
- update non-install project plumbing
- *(core)* decouple stores and implement atomic staging-commit protocol
- *(core)* route build no-targets to warnings
- *(cli)* centralize project bootstrap helpers
- *(app)* preserve resolve failures and presenter fallbacks
- *(app)* model loaded project state explicitly

### Fixed

- *(clippy)* collapse nested if into match guard in BuildPresenter
- refine new command behavior and path normalization

### Other

- Switch release automation to release-plz
- Prepare initial release please version
- Add Apache license metadata and notice
- show docs cache in status output
- Implement JSON-based toolchain resolution
- remove legacy crates and fixtures, consolidate workspace
