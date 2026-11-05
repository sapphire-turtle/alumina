# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/sapphire-turtle/alumina/compare/alumina-datetime-v0.1.0...alumina-datetime-v0.2.0) - 2026-11-05

### Added

- *(alumina-datetime)* the app's icons for the calendar's buttons ([#13](https://github.com/sapphire-turtle/alumina/pull/13))

### Fixed

- *(alumina-datetime)* a chevron icon keeps its aspect ratio ([#13](https://github.com/sapphire-turtle/alumina/pull/13))
- *(alumina-datetime)* non_exhaustive config, icon sizes, and tests that see the icons ([#13](https://github.com/sapphire-turtle/alumina/pull/13))

### Changed

- **breaking** *(alumina-datetime)* `Labels`, `CalendarConfig` and the new `CalendarIcons` are `#[non_exhaustive]` (build them from `default()` and set fields)

### Added

- *(alumina-datetime)* CalendarIcons: the app's icons for the calendar's buttons ([#13](https://github.com/sapphire-turtle/alumina/issues/13))

## [0.1.0](https://github.com/sapphire-turtle/alumina/releases/tag/alumina-datetime-v0.1.0) - 2026-11-03

### Added

- *(alumina-datetime)* DateTimePicker puts the drums under the calendar when narrow
- *(alumina-datetime)* a calendar, time drums and picker buttons ([#7](https://github.com/sapphire-turtle/alumina/pull/7))

### Other

- *(alumina-datetime)* a README and the crates.io metadata
- the workspace: alumina-datetime, alumina-markdown and alumina-demo
