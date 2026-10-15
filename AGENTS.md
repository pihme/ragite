# AGENTS.md

This file is for the coding agent working in this repo. Read it at the start of a session.

<!-- Generated from the presets AGENTS.md template. Fill every TODO, then delete this comment.
     The "Shared rules" section is the same in every repo of the family; change it in presets first. -->

## What this repo is

**ragite** is <!-- TODO: one or two sentences, the same idea as the GitHub repo description -->. Spec and decisions: `SPEC.md`. User docs: `README.md`.

License: **MIT OR Apache-2.0** (dual, Rust convention; `LICENSE-MIT`, `LICENSE-APACHE`; `license = "MIT OR Apache-2.0"` in `Cargo.toml`). MIT copyright line: "ragite contributors"; keep it that way. Not PolyForm.

## How to work here

<!-- TODO: stack and runtime versions, layout (one line per directory), invariants that must not break, what the tests cover. -->

- Tests: `make test` must pass before anything lands on `main`. Tests stay offline: no real accounts, tokens or live services.
- Release paths (only commits touching them can cut a release) are listed in `.github/release.json`.

## Shared rules

### Commits and changes

- [Conventional Commits](https://www.conventionalcommits.org/) for every commit: `feat:`, `fix:`, `perf:`, `docs:`, `test:`, `refactor:`, `chore:`, `ci:`, `build:`, optional scope, `!` for breaking changes.
- **Trivial changes go straight to `main`:** typos, small docs fixes, obvious small bugs, housekeeping. Keep commits small and focused; pull before you commit, CI pushes release commits to `main`.
- **Non-trivial findings become GitHub issues** (with the matching issue form): bugs you do not fix right away, design questions, anything that needs a decision. Do not hide them in a commit.
- **Never force-push**, never rewrite pushed history, never delete branches or tags.
- **No comments, reviews, label changes, closes or merges on other people's issues and PRs** unless the maintainer asked for it in the session. Exception: Dependabot PRs (see below). Opening issues for your own findings and closing them with your own commits (`Closes #n`) is fine.
- **Never write the skip-CI token** (the word `skip` and `ci` in square brackets, or any of its variants) in a commit message, not even quoted or explained: GitHub then skips CI for that push and no release is cut. Only `release.py` puts it in its own release commits.

### Versions and releases

- SemVer, one tag per artifact: `ragite/vX.Y.Z`.
- `feat:` bumps minor, `fix:`/`perf:` patch, `feat!:`/`fix!:` or a `BREAKING CHANGE:` footer major. **Before 1.0.0 a breaking change bumps the minor version** (0.3.x to 0.4.0, never to 1.0.0). Reaching 1.0.0 is a deliberate decision by the maintainer, not a side effect.
- `docs:`, `test:`, `refactor:`, `chore:`, `ci:`, `build:` never bump. A commit only counts when it touches a release path.
- **Release 1.0** (or any chosen version): an empty commit with the footer `Release-As: 1.0.0`, e.g. `git commit --allow-empty -m "chore: release 1.0" -m "Release-As: 1.0.0"`. With several artifacts in `.github/release.json`, name one per footer line: `Release-As: <name>@1.0.0` (the bare form is then ignored). It releases by itself, regardless of type or paths; upwards only (at or below the current version it is ignored with a warning); several footers: the highest wins. Only use it when the maintainer decided the version.
- After CI passes on a push to `main`, `.github/scripts/release.py` computes the version, updates the version file if there is one (commit `chore(release): …` by github-actions), creates the GitHub Release with notes and attaches the build. **Never tag, bump versions or create releases by hand.**

### Dependabot

- Commit prefixes (set in `.github/dependabot.yml`): runtime dependencies and shipped Docker base images `fix(deps):` (patch release), dev dependencies `chore(deps-dev):`, GitHub Actions `ci(deps):`, images that are not shipped (examples, tests) `chore(deps):`.
- **Never turn a dependency update into `feat!:`** or reword its title. A major dependency update is still `fix(deps)` / `chore(deps-dev)`. If it forces a breaking change on users (for example a new minimum runtime), that is a separate, deliberate commit after the maintainer decides.
- **Merge Dependabot PRs when CI is green** (squash, keep the Dependabot title). If one conflicts, comment `@dependabot rebase`. If CI is red and the fix is not obvious, leave the PR open and open an issue.

### Third-party notices

- **Keep `THIRD_PARTY_NOTICES.md` current** (model: git-warden's). It lists everything the project contains, bundles or runs from others: compiled-in or installed dependencies with name, version and license taken from the real dependency metadata (lockfile, `go.mod`, the installed package's `package.json`/LICENSE), the license texts and NOTICE files they ask for, and external programs or images it runs but does not include. Never guess a license. Update it in the same commit as any dependency change (Dependabot included).
- **Ship it with every distributed artifact:** list it in `package.json` `files`, attach it to GitHub Releases next to `LICENSE` (`.github/release.json` assets), and copy it into any image a Dockerfile builds. Link it from the README's license section.
- **The "How this project is built" README section** (verbatim text, `<project>` replaced by the project name) stays in the README, just before the license section; do not reword it. New repos get it from the presets README template (`template/README.md`, copied by `bootstrap.sh`); fill its TODOs, keep the section.

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

<!-- TODO: features, rewrites and dependencies that are out of scope or already decided (list SPEC decisions that must not be reopened). -->

## Agent skills

### Issue tracker

GitHub issues in `pihme/ragite`, via `gh`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default roles: `needs-triage`, `needs-info`, `ready-for-bot`, `ready-for-agent`, `ready-for-qa`, `ready-for-human`, `wontfix`, plus `bug` / `enhancement`. `ready-for-bot` is for in-house bots (Maintainer), `ready-for-agent` is always for an external agent, and `ready-for-qa` is for QA. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: decisions in `SPEC.md`, optional root `GLOSSARY.md`. See `docs/agents/domain.md`.
