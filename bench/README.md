# Model benchmark

Prepared, not yet run. Chooses the default Model among the 11 candidates in SPEC.md section 8.1 (REQ-92). The run starts after the Docling heading fix from PR #11, so the own question set is chunked the way production is.

## 1. Preround (no compute)

`bench/preround/fetch_mteb.py` reads published scores from the data behind the MTEB leaderboard and writes [`preround/mteb_scores.md`](preround/mteb_scores.md) with source commit and date. Leaderboard scores are full-precision PyTorch runs, so they only pre-select; the finalists are measured again through RAGite.

Proposed finalists (maintainer decides): bge-small-en-v1.5, e5-small-v2, nomic-embed-text-v1.5, multilingual-e5-small, plus potion-base-8M as the speed baseline. Decision after the Model2Vec run: potion-retrieval-32M is the built-in default.

## 2. Parity per finalist

`parity/check_parity.py` embeds 300 sentences through RAGite and through the upstream reference (model2vec, or sentence-transformers on PyTorch) and fails below cosine 0.99. The benchmark workflow runs it before every quality run, so a loading or quantisation bug never counts as a weak Model. (The cheap always-on version for the built-in Model is `tests/parity`, run in normal CI.)

## 3. Quality (GitHub Actions, manual)

Workflow **Model benchmark** (`.github/workflows/bench.yml`, `workflow_dispatch`), one Model per run. Datasets: MTEB SciFact, NFCorpus, FiQA2018 (English only), plus the own set in [`questions/`](questions/README.md). Metrics nDCG@10 and Recall@10, each `hybrid` (`MATCH`) and `vector` (KNN on `<name>_vec` only). Pin dataset revisions with `--revision` for the published run.

## 4. CPU costs (fixed machine)

`measure_costs.sh <libragite.so> <out.json>` on the designated 8-core machine, one model at a time and nothing else running: chunks/s while indexing, query latency p50/p95, peak RSS, binary size with the embedded Model, DB size.

## 5. Results

`pareto.py bench/results` writes `RESULTS.md`: commit(s), dataset ids and revisions, own-set sha256, the table, and the Pareto front (quality vs chunks/s and binary size). Pick the smallest Model close to the front.

## How to run

### Start a quality run (GitHub Actions)

Inputs of **Model benchmark**: `model` (a `name` from `candidates.toml`), `datasets` (space-separated MTEB ids, default SciFact, NFCorpus, FiQA), `max_docs` (corpus subsample for a dry run; empty = full).

```sh
gh workflow run bench.yml -f model=potion-base-8M
gh workflow run bench.yml -f model=potion-base-32M -f datasets="mteb/scifact" -f max_docs=500   # dry run
gh run watch "$(gh run list -w bench.yml -L 1 --json databaseId -q '.[0].databaseId')"
gh run download <run-id> -n bench-<model> -D bench/results
```

One Model per run; start the next when the previous one is done to keep CI minutes low. The parity step runs first and fails the run before any quality numbers are produced.

### Parity and costs locally

```sh
pip install -r bench/requirements.txt
ext=$(bench/build_model.sh minishlab/potion-base-8M potion-base-8M)
python3 bench/parity/check_parity.py --extension "$ext" --hf minishlab/potion-base-8M --kind model2vec
bench/measure_costs.sh "$ext" bench/results/costs/potion-base-8M.json
```

Costs only mean something on a quiet machine, one Model at a time. Large Models (potion-multilingual-128M, 512 MB weights) need about 3 GB free RAM to build.

## Where results land and how to read them

- `bench/results/quality/<model>--<dataset>.json`: nDCG@10 and Recall@10 per mode (`hybrid`, `vector`), latency, chunks/s, dataset id and revision, commit.
- `bench/results/costs/<model>.json`: binary and DB size, peak RSS, chunks/s, query latency p50/p95, machine.
- `python3 bench/pareto.py bench/results` writes `bench/RESULTS.md`.

In the table, quality is the mean hybrid nDCG@10 over all datasets; the vector column shows what the embedding alone contributes (hybrid minus vector is what full-text search adds). A Model is on the **Pareto front** when no other Model is at least as good in quality and at least as cheap in chunks/s and binary size, and strictly better in one. Off-front Models are dominated; pick the smallest front Model whose quality is close to the best.

## Why hybrid is close to vector-only

In the results, hybrid and vector-only nDCG@10 are nearly equal, and Recall@10 on SciFact is identical. That is expected, not a bug: per REQ-75/REQ-61 the FTS5 side searches the whole query as one exact phrase, and benchmark questions almost never occur verbatim in a document. Check on SciFact with potion-base-8M (300 queries, 23,246 Chunks, all in the FTS index):

| FTS query form | queries with FTS hits | FTS-only nDCG@10 | hybrid nDCG@10 / Recall@10 |
| --- | --- | --- | --- |
| exact phrase (RAGite, REQ-75) | 1 / 300 | 0.0033 | 0.5255 / 0.6638 |
| OR of terms (plain BM25, for comparison) | 300 / 300 | 0.6093 | 0.6200 / 0.7799 |

The FTS leg helps only for queries that quote document text verbatim. Exact-phrase search is kept by design; the numbers measure the vector Model.

## Known gap

RAGite loads only Model2Vec models today (`model2vec-rs`). The transformer finalists need the candle loader (REQ-22) before steps 2-4 can run for them; `build_model.sh` and the workflow will fail for them until then.
