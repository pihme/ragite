#!/usr/bin/env python3
"""Quality run for one Model through ragite's own path (REQ-92).

Loads an extension built with that Model (see bench/README.md), indexes a
retrieval dataset through `INSERT`, and scores nDCG@10 and Recall@10 in two
modes:

  hybrid  -- `docs_chunks MATCH q` (sqlite-vec + FTS5, the ragite default)
  vector  -- KNN on `docs_vec` with `ragite_embed(..., 'query')` only

A document's score is the best rank of any of its Chunks. Output is one JSON
file with the commit, the Model id, dataset ids and revisions, and the
sha256 of the frozen question file.

    python3 bench/run_quality.py --extension target/release/libragite.so \
        --dataset mteb/scifact --out bench/results/scifact.json
    python3 bench/run_quality.py --extension ... --own bench/questions/own.jsonl ...

Needs `pip install -r bench/requirements.txt` for MTEB datasets. Not run in normal CI.
"""
import argparse
import hashlib
import json
import math
import pathlib
import sqlite3
import subprocess
import time

K = 10


def load_mteb(name: str, revision: str | None, max_docs: int | None):
    from datasets import load_dataset  # heavy import only when needed

    corpus = load_dataset(name, "corpus", revision=revision, split="corpus")
    queries = load_dataset(name, "queries", revision=revision, split="queries")
    qrels = load_dataset(name, "default", revision=revision, split="test")
    rel: dict[str, set[str]] = {}
    for r in qrels:
        if float(r["score"]) > 0:
            rel.setdefault(str(r["query-id"]), set()).add(str(r["corpus-id"]))
    docs = [(str(d["_id"]), ((d.get("title") or "") + "\n\n" + d["text"]).strip()) for d in corpus]
    if max_docs:  # keep every relevant doc, then fill up
        needed = set().union(*rel.values())
        docs = [d for d in docs if d[0] in needed] + [d for d in docs if d[0] not in needed][: max(0, max_docs - len(needed))]
    qs = [(str(q["_id"]), q["text"]) for q in queries if str(q["_id"]) in rel]
    return docs, qs, rel, {"id": name, "revision": revision or "main"}


def load_own(path: pathlib.Path):
    """Own set: JSONL rows {"id", "question", "relevant": [source,...], "lang"};
    documents are DoclingDocument JSON files in bench/questions/docs/."""
    rows = [json.loads(l) for l in path.read_text().splitlines() if l.strip()]
    docs = [(p.name, p.read_text()) for p in sorted((path.parent / "docs").glob("*.json"))]
    rel = {r["id"]: set(r["relevant"]) for r in rows}
    qs = [(r["id"], r["question"]) for r in rows]
    return docs, qs, rel, {"id": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def ndcg(ranked, relevant):
    dcg = sum(1 / math.log2(i + 2) for i, d in enumerate(ranked[:K]) if d in relevant)
    ideal = sum(1 / math.log2(i + 2) for i in range(min(K, len(relevant))))
    return dcg / ideal if ideal else 0.0


def dedupe(sources):
    seen, out = set(), []
    for s in sources:
        if s not in seen:
            seen.add(s)
            out.append(s)
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--extension", required=True)
    ap.add_argument("--dataset", help="MTEB/BEIR dataset on Hugging Face, e.g. mteb/scifact")
    ap.add_argument("--revision")
    ap.add_argument("--own", type=pathlib.Path, help="own question set (JSONL)")
    ap.add_argument("--max-docs", type=int, help="subsample corpus (dry runs only)")
    ap.add_argument("--max-queries", type=int)
    ap.add_argument("--db", default=":memory:")
    ap.add_argument("--out", required=True, type=pathlib.Path)
    a = ap.parse_args()

    docs, qs, rel, dataset = load_own(a.own) if a.own else load_mteb(a.dataset, a.revision, a.max_docs)
    fmt = "application/vnd.docling.document+json" if a.own else "text/plain"
    if a.max_queries:
        qs = qs[: a.max_queries]

    con = sqlite3.connect(a.db)
    con.enable_load_extension(True)
    con.load_extension(a.extension)
    model = con.execute("SELECT technical_id FROM ragite_default_model()").fetchone()[0]
    con.execute("CREATE VIRTUAL TABLE docs USING ragite")
    t0 = time.perf_counter()
    with con:
        con.executemany("INSERT INTO docs(text, source, format) VALUES (?, ?, ?)", [(t, s, fmt) for s, t in docs])
    index_s = time.perf_counter() - t0
    chunks = con.execute("SELECT count(*) FROM docs_chunks").fetchone()[0]

    depth = 50  # enough Chunks to fill 10 distinct documents in most cases
    modes = {
        "hybrid": "SELECT source FROM docs_chunks WHERE docs_chunks MATCH ? LIMIT %d" % depth,
        "vector": (
            "SELECT c.source FROM docs_vec v JOIN docs_chunks c ON c.rowid = v.rowid "
            "WHERE v.embedding MATCH ragite_embed('docs', ?, 'query') AND k = %d ORDER BY v.distance" % depth
        ),
    }
    result = {"model": model, "dataset": dataset, "docs": len(docs), "queries": len(qs),
              "chunks": chunks, "index_seconds": round(index_s, 3), "chunks_per_second": round(chunks / index_s, 1),
              "commit": subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip(),
              "modes": {}}
    for mode, sql in modes.items():
        n, r, lat = [], [], []
        for qid, text in qs:
            t = time.perf_counter()
            ranked = dedupe(row[0] for row in con.execute(sql, (text,)))
            lat.append(time.perf_counter() - t)
            n.append(ndcg(ranked, rel[qid]))
            r.append(len(set(ranked[:K]) & rel[qid]) / len(rel[qid]))
        lat.sort()
        result["modes"][mode] = {"ndcg@10": round(sum(n) / len(n), 4), "recall@10": round(sum(r) / len(r), 4),
                                 "latency_ms_p50": round(1000 * lat[len(lat) // 2], 2),
                                 "latency_ms_p95": round(1000 * lat[int(len(lat) * 0.95)], 2)}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
