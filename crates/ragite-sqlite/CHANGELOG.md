# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0](https://github.com/pihme/ragite/compare/ragite/v0.3.3...ragite/v1.0.0) - 2026-10-18

### Fixed

- name RAGite in error messages and docs ([#86](https://github.com/pihme/ragite/pull/86))

## [0.3.3](https://github.com/pihme/ragite/compare/ragite/v0.3.2...ragite/v0.3.3) - 2026-10-18

### Other

- keep benchmark inputs out of the shell ([#75](https://github.com/pihme/ragite/pull/75))
- keep a Chunk's three tables behind one module ([#72](https://github.com/pihme/ragite/pull/72))
- give MATCH one Search module in ragite-sqlite ([#69](https://github.com/pihme/ragite/pull/69))
- recognize a Document table where the companion marker is written ([#70](https://github.com/pihme/ragite/pull/70))
- release v0.3.2 ([#62](https://github.com/pihme/ragite/pull/62))

## [0.3.2](https://github.com/pihme/ragite/compare/ragite/v0.3.1...ragite/v0.3.2) - 2026-10-18

### Fixed

- apply = and IN before the fusion depth ([#57](https://github.com/pihme/ragite/pull/57))

### Other

- correct REQ-15 so LIMIT stays on the fused rows ([#59](https://github.com/pihme/ragite/pull/59))
- specify the sqlite-vec 0.1.9 surface ([#60](https://github.com/pihme/ragite/pull/60))

## [0.3.1](https://github.com/pihme/ragite/compare/ragite/v0.3.0...ragite/v0.3.1) - 2026-10-18

### Other

- address QA notes on the user guide and its block test ([#48](https://github.com/pihme/ragite/pull/48))
- add a user guide whose SQL examples run in CI ([#44](https://github.com/pihme/ragite/pull/44))
- gate the release PR on release-inputs and record Model changes in ragite-core ([#45](https://github.com/pihme/ragite/pull/45))
- list root release inputs and perf commits in the changelog ([#42](https://github.com/pihme/ragite/pull/42))
- working quickstart, setup prerequisites, and tests that keep them honest ([#41](https://github.com/pihme/ragite/pull/41))

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
