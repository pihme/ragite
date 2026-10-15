# Roadmap

ragite is pre-1.0. This file lists what each major version is meant to deliver. Details and requirement IDs live in [SPEC.md](SPEC.md).

## Version 1.0

Version 1.0 delivers everything specified in [SPEC.md](SPEC.md) (REQ-1 to REQ-27): the SQLite extension with its virtual table interface (INSERT, UPDATE, DELETE, search), source linkage through documents and chunks, the three model variants, and the licensing and NOTICE handling for models. The default model is chosen by benchmark (SPEC section 8.1).

## Version 2

- **Own Rust crate:** the Rust core is published as its own package, usable without SQLite.
- **Language bindings covered by tests:** use from Python, C, Go, Java and Node.js, each verified by automated tests.
- **Statistics on stored documents:** the number of stored documents and their sizes can be queried.
