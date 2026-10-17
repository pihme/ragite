# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/pihme/ragite/compare/ragite-core/v0.2.0...ragite-core/v0.3.0) - 2026-10-16

### Added

- mark the switch to potion-retrieval-32M (0.2.0, [#32](https://github.com/pihme/ragite/pull/32)) as breaking; store the tokenizers in Git LFS ([#34](https://github.com/pihme/ragite/pull/34))

### Fixed

- bound a Docling table by its cells ([#25](https://github.com/pihme/ragite/pull/25))
- assemble the model weights on macOS and Windows ([#38](https://github.com/pihme/ragite/pull/38))

### Other

- lock the non-equivalent mutation survivors ([#33](https://github.com/pihme/ragite/pull/33))

## [0.2.0](https://github.com/pihme/ragite/compare/ragite-core/v0.1.1...ragite-core/v0.2.0) - 2026-10-15

### Added

- [**breaking**] make potion-retrieval-32M the built-in default Model ([#32](https://github.com/pihme/ragite/pull/32))
  Existing databases indexed with the old default potion-base-8M (dimension 256) must be re-indexed: the built-in Model now has a different technical id and dimension 512. The extension grows from ~36 MB to ~131 MB.

### Fixed

- store numeric SQL values as their text ([#18](https://github.com/pihme/ragite/pull/18))

## [0.1.1](https://github.com/pihme/ragite/compare/ragite-core/v0.1.0...ragite-core/v0.1.1) - 2026-10-15

### Fixed

- build large provider Models without OOM in the benchmark ([#15](https://github.com/pihme/ragite/pull/15))

## [0.1.0](https://github.com/pihme/ragite/releases/tag/ragite-core/v0.1.0) - 2026-10-15

### Added

- search documents with a SQLite virtual table ([#11](https://github.com/pihme/ragite/pull/11))
