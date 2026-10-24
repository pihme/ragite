# gh-pages source

This branch holds the static GitHub Pages site for `pihme/ragite`.

Generated on 2026-10-24 from `main` at commit `8759f4db92a8f6e7a9cabb6e5f27f2f3398adf35`.
Self-contained `index.html`, `jigsaw/index.html` (family page, from `family.json`) and, where present, `namesake/index.html` (from `namesake-<repo>.json`), all with inline CSS, no build step, no trackers; plus favicons (`favicon.svg`, `favicon.png`, `apple-touch-icon.png`) and `.nojekyll`.
Theme `pair` (`theme_pair.py`), with self-hosted fonts in `assets/` (SIL OFL 1.1, licence texts alongside), plus `site.webmanifest` with 192/512 icons and, where the site has its own icons, `favicon.ico`, `og-image.png` and `assets/logo/`.
Generator: `gen.py` (layout B, sidebar handbook) + per-site content script + `family.json` project-family registry + `status-<repo>.json` (Current status; open issues and releases pulled live via `gh` at build time).
Regenerate when `main` changes; do not merge this branch into `main`.
