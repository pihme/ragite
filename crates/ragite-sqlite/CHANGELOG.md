# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/pihme/ragite/compare/ragite/v0.2.0...ragite/v0.3.0) - 2026-10-16

### Added

- [**breaking**] default Model potion-retrieval-32M is breaking; tokenizers in LFS ([#34](https://github.com/pihme/ragite/pull/34))

### Fixed

- assemble the model weights on macOS and Windows ([#38](https://github.com/pihme/ragite/pull/38))
- bound a Docling table by its cells ([#25](https://github.com/pihme/ragite/pull/25))

### Other

- look up a chunk by rowid ([#26](https://github.com/pihme/ragite/pull/26))
- lock the non-equivalent mutation survivors ([#33](https://github.com/pihme/ragite/pull/33))

## [0.2.0](https://github.com/pihme/ragite/compare/ragite/v0.1.1...ragite/v0.2.0) - 2026-10-15

### Added

- make potion-retrieval-32M the built-in default Model ([#32](https://github.com/pihme/ragite/pull/32))

### Fixed

- store numeric SQL values as their text ([#18](https://github.com/pihme/ragite/pull/18))

## [0.1.1](https://github.com/pihme/ragite/compare/ragite/v0.1.0...ragite/v0.1.1) - 2026-10-15

### Fixed

- build large provider Models without OOM in the benchmark ([#15](https://github.com/pihme/ragite/pull/15))

## [0.1.0](https://github.com/pihme/ragite/releases/tag/ragite/v0.1.0) - 2026-10-15

### Added

- search documents with a SQLite virtual table ([#11](https://github.com/pihme/ragite/pull/11))
