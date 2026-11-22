# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/sunsided/search-core/compare/v0.3.0...v0.3.1) - 2026-11-22

### Changed

- Link the crate to its page on the planning and search site ([#17](https://github.com/sunsided/search-core/pull/17))
- Describe the released scope and audience in the README ([#15](https://github.com/sunsided/search-core/pull/15))
- Link the crate to its page on the planning and search site
- Describe the released scope and audience in the README

## [0.3.0](https://github.com/sunsided/search-core/compare/v0.2.0...v0.3.0) - 2026-11-21

### Added

- Add by-reference and in-place BigInt and BigUint operators

## [0.2.0](https://github.com/sunsided/search-core/compare/v0.1.0...v0.2.0) - 2026-11-21

### Added

- Add BigInt, an exact signed integer for cost expressions
- Add Lexicographic, a pair cost with a tie-breaker

### Changed

- Rewrite the README for the 0.1 release

## [0.1.0](https://github.com/sunsided/search-core/compare/v0.0.0...v0.1.0) - 2026-11-21

### Added

- Implement the blind Zero heuristic for f64 ([#3](https://github.com/sunsided/search-core/pull/3))
- Add CostExt, priority arithmetic for costs ([#4](https://github.com/sunsided/search-core/pull/4))
- Allow merge-queue branches in the branch check
- Add CostExt, priority arithmetic for costs
- Implement the blind Zero heuristic for f64
- Add FromStr for Decimal
- Add release-plz and the crates.io publish workflow
- Add git hooks, Copilot review rules and CONTRIBUTING.md
- Add badges and install, features and MSRV sections to the README
- Add a fuzz target and criterion benches
- Add REUSE metadata and license texts
- Add cargo-deny, Dependabot and typos
- Add Codecov configuration
- Add Taskfile and CI workflows
- Add ExactArith, exact arithmetic of any size for costs
- Add WideMul, an exact cost-by-cost product
- Add the shared Cost, Heuristic, Goal and Budget traits

### Changed

- Set the version to 0.1.0 for the first release
- Pin dtolnay/rust-toolchain to its master branch
- Anchor the attribution trailers in the commit-message guard
- Document the Decimal scaling on CostExt::to_f64 and mark it must_use
- Describe the shared Zero in the stochastic-search stand-in docs
- Report overlong fractions consistently when parsing Decimal
- Report unclosed mermaid blocks instead of skipping them
- Record Decimal parsing as resolved in MIGRATION.md
- Enforce the attribution guard through the hook config
- Build the fuzz targets for the gnu host in CI
- Keep repository tooling out of the published crate
- Tighten lints and add clippy and rustfmt configuration
- Doctest the README and render mermaid on docs.rs
- Document scope and the sibling migration plan
- Test the traits from a downstream crate

### Fixed

- Fix stale references in tooling comments
- Fix feature-gated doc links and cover float and weighted keys
