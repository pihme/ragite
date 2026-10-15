# Contributing

ragite is open source, dual-licensed under the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option.

## Issues

Issues, bug reports and ideas are very welcome. Pick the matching [issue form](https://github.com/pihme/ragite/issues/new/choose) (bug report, feature request, documentation, question). A good report names the ragite version or commit, your OS and runtime versions, what you ran, what you expected and what happened.

## Pull requests

Pull requests are welcome; unless you state otherwise, any contribution you submit is dual-licensed as above (MIT OR Apache-2.0), without additional terms. For anything bigger than a small fix, please open an issue first so we can agree on the change. Keep a pull request to one change, fill in the [pull request template](.github/pull_request_template.md), give it a [Conventional Commit](https://www.conventionalcommits.org/) title (`fix: …`, `feat: …`, `docs: …`; it becomes the squash commit and decides the next release), and make sure `make test` passes.

## Chunker, scoring, and other presets

Changes to the chunker, the scoring algorithm, and the other fixed presets are very welcome. See [Limitations](README.md#limitations). A pull request that comes with data receives preferential treatment.

## Security and secrets

The tracker is public. Never paste tokens, keys, credentials or real service responses into an issue or pull request. Please report vulnerabilities privately, as described in [SECURITY.md](SECURITY.md).

## Build and test locally

<!-- TODO: requirements (runtime versions, tools) and anything the tests need or do not need. -->

```bash
make test
```

Test cases are numbered in [TESTSPEC.md](TESTSPEC.md). Cite those numbers from the tests.
