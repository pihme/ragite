# Roadmap

ragite is pre-1.0. This file lists what each major version is meant to deliver. Details and requirement IDs live in [SPEC.md](SPEC.md).

Statements about future versions reflect the current line of thinking. They are not a commitment.

## Version 1.0

Version 1.0 delivers everything specified in [SPEC.md](SPEC.md): the SQLite extension with its virtual table interface (INSERT, UPDATE, DELETE, search), source linkage through documents and chunks, a built-in model or a model linked at build time, and the licensing and NOTICE handling for models. The default model is chosen by benchmark (SPEC section 8.1). A model is not loaded from a path or a URL at run time.

## Version 2.0

- **Own Rust crate:** the Rust core is published as its own package, usable without SQLite.
- **Language bindings covered by tests:** use from Python, C, Go, Java and Node.js, each verified by automated tests.
- **Statistics on stored documents:** the number of stored documents and their sizes can be queried.
- **Search:** search will likely become more sophisticated than version 1.0's fixed Reciprocal Rank Fusion and its fixed candidate depth.
- **Formats:** format handling will be improved beyond version 1.0's four media types.
