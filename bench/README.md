# Model benchmark

Prepared, not yet run. Chooses the default Model among the 11 candidates in SPEC.md section 8.1 (REQ-89). The run starts after the Docling heading fix from PR #11, so the own question set is chunked the way production is.

## 1. Preround (no compute)

`bench/preround/fetch_mteb.py` reads published scores from the data behind the MTEB leaderboard and writes [`preround/mteb_scores.md`](preround/mteb_scores.md) with source commit and date. Leaderboard scores are full-precision PyTorch runs, so they only pre-select; the finalists are measured again through ragite.

Proposed finalists (maintainer decides): bge-small-en-v1.5, e5-small-v2, nomic-embed-text-v1.5, multilingual-e5-small (best German score among small models), plus potion-base-8M as the current default and speed baseline.

## 2. Parity per finalist

`parity/check_parity.py` embeds 300 sentences through ragite and through the upstream reference (model2vec, or sentence-transformers on PyTorch) and fails below cosine 0.99. The benchmark workflow runs it before every quality run, so a loading or quantisation bug never counts as a weak Model. (The cheap always-on version for the built-in Model is `tests/parity`, run in normal CI.)

## 3. Quality (GitHub Actions, manual)

Workflow **Model benchmark** (`.github/workflows/bench.yml`, `workflow_dispatch`), one Model per run. Datasets: MTEB SciFact, NFCorpus, FiQA2018, GermanQuAD-Retrieval, plus the own set in [`questions/`](questions/README.md). Metrics nDCG@10 and Recall@10, each `hybrid` (`MATCH`) and `vector` (KNN on `<name>_vec` only). Pin dataset revisions with `--revision` for the published run.

## 4. CPU costs (fixed machine)

`measure_costs.sh <libragite.so> <out.json>` on the designated 8-core machine, one model at a time and nothing else running: chunks/s while indexing, query latency p50/p95, peak RSS, binary size with the embedded Model, DB size.

## 5. Results

`pareto.py bench/results` writes `RESULTS.md`: commit(s), dataset ids and revisions, own-set sha256, the table, and the Pareto front (quality vs chunks/s and binary size). Pick the smallest Model close to the front.

## Known gap

ragite loads only Model2Vec models today (`model2vec-rs`). The transformer finalists need the candle loader (REQ-22) before steps 2-4 can run for them; `build_model.sh` and the workflow will fail for them until then.
