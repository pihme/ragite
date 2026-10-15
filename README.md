# ragite

Embeddable, CPU-friendly retrieval (RAG) as a SQLite extension.

Status: pre-1.0, specification only. No code yet.

Website: https://pihme.github.io/ragite/

## Requirements

<!-- TODO: runtime and tools with minimum versions (e.g. Node.js 22+, Go 1.24+, Docker), OS/arch limits. -->

## Getting started

```bash
git clone https://github.com/pihme/ragite.git
cd ragite
# TODO: install / build steps
make test
```

<!-- TODO: smallest working example (hello world), then a link to the usage docs. Releases: https://github.com/pihme/ragite/releases -->

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: [SECURITY.md](SECURITY.md).

## How this project is built

ragite is developed agent-first. AI coding agents write all code,
tests, and documentation, and review each other's changes, under human
direction: specs, design decisions, and acceptance based on observed
behaviour and test results. No human reads the code line by line. This
is a deliberate choice. Quality rests on automated tests, CI, and
independent agent review.

Evaluate the code against your own requirements before you depend on it.
Found a problem? Open an issue.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option (SPDX: `MIT OR Apache-2.0`). Third-party software and its licenses: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
