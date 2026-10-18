# AGENTS.md

This file is for the coding agent working in this repo. Read it at the start of a session.

## What this repo is

**RAGite** is embeddable, CPU-friendly retrieval (RAG) as a SQLite extension. Spec and decisions: `SPEC.md`. User docs: `README.md`.

License: **MIT OR Apache-2.0** (dual, Rust convention; `LICENSE-MIT`, `LICENSE-APACHE`; `license = "MIT OR Apache-2.0"` in `Cargo.toml`). MIT copyright line: "ragite contributors"; keep it that way. Not PolyForm.

## How to work here

- Rust edition 2024, `rust-version` 1.88. `crates/ragite-core` is the core, kept separate from the SQLite layer (REQ-1). `crates/ragite-sqlite` is the loadable extension. The built-in Model is `models/potion-retrieval-32M/`. `bench/` is the manual Model benchmark. `tests/languages/` loads the extension from Python, Node, and Go 1.22.
- Tests: `make test` must pass before anything lands on `main`. Tests stay offline: no real accounts, tokens or live services.
- **Releases use release-plz, not `release.py`.** There is no `.github/release.json` and no `Release-As:` footer. Config: `release-plz.toml`. Both crates stay `publish = false` (`git_only`): nothing goes to crates.io. After CI passes on a push to `main`, release-plz opens a release pull request that bumps the crate versions and writes the changelogs. Merging that pull request creates the tags and the GitHub Release, and attaches `libragite.so`, `LICENSE-MIT`, `LICENSE-APACHE`, and `THIRD_PARTY_NOTICES.md`. Bump rules are in Versions and releases below. Do not tag, bump versions, or create releases by hand.
- **release-plz only lists commits that touch a crate directory** (`crates/ragite-core`, `crates/ragite-sqlite`). A change that affects the shipped extension from outside them (the release profile in the workspace `Cargo.toml`, the built-in Model in `models/potion-retrieval-32M/`) must also run `make release-inputs` and commit the regenerated `crates/ragite-sqlite/release-inputs.txt` (workspace `Cargo.toml`) and `crates/ragite-core/release-inputs.txt` (the Model, which ragite-core embeds); CI fails otherwise, and the release pull request is not updated. Use `perf:` for size or speed changes (own changelog group) and `!` plus a `BREAKING CHANGE:` footer for anything that forces users to re-index or change code, in the PR that makes the change, not in a follow-up. Commit titles become the changelog lines verbatim, so write them as the change itself ("make X the default"), without repeating "breaking".

## Shared rules

### Commits and changes

- [Conventional Commits](https://www.conventionalcommits.org/) for every commit: `feat:`, `fix:`, `perf:`, `docs:`, `test:`, `refactor:`, `chore:`, `ci:`, `build:`, optional scope, `!` for breaking changes.
- **Trivial changes go straight to `main`:** typos, small docs fixes, obvious small bugs, housekeeping. Keep commits small and focused; pull before you commit.
- **Non-trivial findings become GitHub issues** (with the matching issue form): bugs you do not fix right away, design questions, anything that needs a decision. Do not hide them in a commit.
- **Never force-push**, never rewrite pushed history, never delete branches or tags.
- **No comments, reviews, label changes, closes or merges on other people's issues and PRs** unless the maintainer asked for it in the session. Exception: Dependabot PRs (see below). Opening issues for your own findings and closing them with your own commits (`Closes #n`) is fine.
- **Never write the skip-CI token** (the word `skip` and `ci` in square brackets, or any of its variants) in a commit message, not even quoted or explained: GitHub then skips CI for that push and no release is cut.

### Pull request workflow

- **The `ready-for-qa` label is the only readiness signal.** Chat messages do not count. Before pushing further commits to a PR, remove `ready-for-qa`; add it back once the push is done and CI is green.
- **QA picks up `ready-for-qa` PRs on its own.** No ping is needed.
- **After a rebase, QA reviews only the delta** with `git range-diff`. When the rebase only resolved conflicts, say so in the PR.
- **Long-running jobs run in the background or at the end** of a work step. Terminate orphaned runs (cancelled work, superseded commits).
- **PRs that touch shared files** (`TESTSPEC.md`, `CONTRIBUTING.md`, `THIRD_PARTY_NOTICES.md`) are merged one at a time, smallest first. After each merge, update the other open PRs to `main` right away. Always base a new PR on the current `main`.

### Versions and releases

- SemVer. `ragite-sqlite` is tagged `ragite/vX.Y.Z`. `ragite-core` is tagged `ragite-core/vX.Y.Z`. The GitHub Release is named `RAGite X.Y.Z` (releases before 1.0 keep their name `ragite X.Y.Z`).
- `feat:` bumps minor, including before 1.0. `fix:` and `perf:` bump patch. A breaking change before 1.0 bumps minor (0.3.x to 0.4.0, never to 1.0.0). Choose 1.0.0 by editing the version in the release pull request. After 1.0, `feat!:`, `fix!:`, or a `BREAKING CHANGE:` footer bumps major.
- `docs:`, `test:`, `refactor:`, `chore:`, `ci:`, and `build:` never bump. A commit counts only when it touches `crates/ragite-core` or `crates/ragite-sqlite`, or when `make release-inputs` updates a `release-inputs.txt` for an input outside those directories.
- Release-plz opens the release pull request after CI passes on `main`. Merging it tags and publishes. Do not tag, bump versions, or create releases by hand.

### Dependabot

- `.github/dependabot.yml` opens the updates. Workspace Cargo (`/`) is `fix(deps):` and bumps patch, because those crates are release paths. The fuzz Cargo workspace (`/fuzz`), the Go language tests (`/tests/languages/go`), and the bench pip dependencies (`/bench`) are `chore(deps):` and are not shipped. GitHub Actions are `ci(deps):`.
- **Never turn a dependency update into `feat!:`** or reword its title. A major update keeps the prefix from `.github/dependabot.yml`. If it forces a breaking change on users (for example a new minimum runtime), that is a separate, deliberate commit after the maintainer decides.
- **Merge Dependabot PRs when CI is green** (squash, keep the Dependabot title). If one conflicts, comment `@dependabot rebase`. If CI is red and the fix is not obvious, leave the PR open and open an issue.

### Third-party notices

- **Keep `THIRD_PARTY_NOTICES.md` current** (model: git-warden's). It lists everything the project contains, bundles or runs from others: compiled-in or installed dependencies with name, version and license taken from the real dependency metadata (`Cargo.lock` and `cargo metadata`, and `go.mod` for the language test), the license texts and NOTICE files they ask for, and external programs or images it runs but does not include. Never guess a license. Update it in the same commit as any dependency change (Dependabot included).
- **Ship it with the GitHub Release.** The release job uploads `THIRD_PARTY_NOTICES.md` next to `libragite.so`, `LICENSE-MIT`, and `LICENSE-APACHE`. Link it from the README's license section. This repo has no `package.json` and no Dockerfile.
- **The "How this project is built" README section** stays in the README, just before the license section; do not reword it.

### Secrets

- Never commit, print, log or paste tokens, API keys, credentials, `.env` files, session files or real service responses: not in code, tests, fixtures, issues, PRs or commit messages. Use synthetic fixtures.
- Security problems are reported privately (`SECURITY.md`). Do not discuss an unfixed vulnerability in a public issue.

### Contributions

- Outside pull requests are welcome under MIT OR Apache-2.0 (see `CONTRIBUTING.md` and `.github/pull_request_template.md`). The maintainer reviews and merges them (squash, Conventional Commit title).
- `CONTRIBUTING.md` holds **only content for outside people**: license, how to report, how to build and test. Internal working rules (this file) never go there.

### Website

- Site: <https://pihme.github.io/ragite/>, generated onto the `gh-pages` branch from outside this repo. Do not edit `gh-pages` by hand and never merge it into `main`.
- **Repo description = site tagline.** The hero tagline is the live GitHub repo description; change them together (the generator checks it).
- **Current status:** two link-free prose paragraphs, *what works* and *limits* (limits include the open issues, described in words, never as numbers or links). About 90–130 words each, plain English for someone who does not know the project, acronyms explained or avoided. No dates, SHAs or "as of" in the prose: metadata appears once, in the line `Last updated <date> · main@<sha>`. Only state what tests, CI, releases or the code show. The covered issues must match the open issues exactly.
- **Chronicles:** one paragraph per month, newest first, about 60–90 words, only the main milestones. Written as a modest medieval chronicler: the author is always *a humble brother* (or a monk of the order), never "the maker". Light archaic touch, plain verbs, no clock times, minimal dates, no superlatives or flourishes, facts only (never mention things that do not exist yet), one to three links to releases, commits or compares.
- **Jigsaw:** the project family lives in one central `family.json`. Each relation is described truthfully from both sides; a new project, a changed description or a changed relation means rebuilding all sites and the profile README.

## Do not invent

Decisions marked "Do not reopen" are in `SPEC.md`. Do not reopen them, and do not copy that list into this file.

## Agent skills

### Issue tracker

GitHub issues in `pihme/ragite`, via `gh`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default roles: `needs-triage`, `needs-info`, `ready-for-bot`, `ready-for-agent`, `ready-for-qa`, `ready-for-human`, `wontfix`, plus `bug` / `enhancement`. `ready-for-bot` is for in-house bots (Maintainer), `ready-for-agent` is always for an external agent, and `ready-for-qa` is for QA. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: decisions in `SPEC.md`, optional root `GLOSSARY.md`. See `docs/agents/domain.md`.
