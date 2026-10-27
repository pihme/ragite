# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0](https://github.com/pihme/ragite/compare/ragite-core/v1.0.1...ragite-core/v1.1.0) - 2026-10-27

### Added

- drop vector neighbors beyond a fixed cosine distance ([#116](https://github.com/pihme/ragite/pull/116))
- split a long unit on a word boundary ([#112](https://github.com/pihme/ragite/pull/112))

## [1.0.1](https://github.com/pihme/ragite/compare/ragite-core/v1.0.0...ragite-core/v1.0.1) - 2026-10-19

### Fixed

- *(deps)* bump sha2 from 0.10.9 to 0.11.1 ([#78](https://github.com/pihme/ragite/pull/78))

## [1.0.0](https://github.com/pihme/ragite/compare/ragite-core/v0.3.3...ragite-core/v1.0.0) - 2026-10-18

### Fixed

- name RAGite in error messages and docs ([#86](https://github.com/pihme/ragite/pull/86))

## [0.3.3](https://github.com/pihme/ragite/compare/ragite-core/v0.3.2...ragite-core/v0.3.3) - 2026-10-18

### Other

- release v0.3.2 ([#62](https://github.com/pihme/ragite/pull/62))

## [0.3.2](https://github.com/pihme/ragite/compare/ragite-core/v0.3.1...ragite-core/v0.3.2) - 2026-10-18

### Other

- release v0.3.2 ([#61](https://github.com/pihme/ragite/pull/61))

## [0.3.1](https://github.com/pihme/ragite/compare/ragite-core/v0.3.0...ragite-core/v0.3.1) - 2026-10-18

### Other

- gate the release PR on release-inputs and record Model changes in ragite-core ([#45](https://github.com/pihme/ragite/pull/45))

## [0.3.0](https://github.com/pihme/ragite/compare/ragite-core/v0.2.0...ragite-core/v0.3.0) - 2026-10-16

### Added

- [**breaking**] default Model potion-retrieval-32M is breaking; tokenizers in LFS ([#34](https://github.com/pihme/ragite/pull/34))

### Fixed

- bound a Docling table by its cells ([#25](https://github.com/pihme/ragite/pull/25))
- assemble the model weights on macOS and Windows ([#38](https://github.com/pihme/ragite/pull/38))

### Other

- lock the non-equivalent mutation survivors ([#33](https://github.com/pihme/ragite/pull/33))

## [0.2.0](https://github.com/pihme/ragite/compare/ragite-core/v0.1.1...ragite-core/v0.2.0) - 2026-10-15

### Added

- make potion-retrieval-32M the built-in default Model ([#32](https://github.com/pihme/ragite/pull/32))

### Fixed

- store numeric SQL values as their text ([#18](https://github.com/pihme/ragite/pull/18))

## [0.1.1](https://github.com/pihme/ragite/compare/ragite-core/v0.1.0...ragite-core/v0.1.1) - 2026-10-15

### Fixed

- build large provider Models without OOM in the benchmark ([#15](https://github.com/pihme/ragite/pull/15))

## [0.1.0](https://github.com/pihme/ragite/releases/tag/ragite-core/v0.1.0) - 2026-10-15

### Added

- search documents with a SQLite virtual table ([#11](https://github.com/pihme/ragite/pull/11))
