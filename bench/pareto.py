#!/usr/bin/env python3
"""Merge quality and cost results into bench/RESULTS.md with the Pareto front.

    python3 bench/pareto.py bench/results

Reads <dir>/quality/*.json (from run_quality.py) and <dir>/costs/*.json (from
measure_costs.sh). Quality = mean hybrid nDCG@10 over the English datasets;
GermanQuAD (optional) is shown in its own column and does not count. A Model is
on the front when no other Model is at least as good in quality and at least as
cheap in both chunks/s and binary size, and strictly better in one.
"""
import collections
import json
import pathlib
import sys

d = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "bench/results")
q = collections.defaultdict(dict)
meta = collections.defaultdict(set)
for f in sorted((d / "quality").glob("*.json")):
    r = json.loads(f.read_text())
    q[r["model"]][r["dataset"]["id"]] = r
    meta["datasets"].add(json.dumps(r["dataset"], sort_keys=True))
    meta["commits"].add(r["commit"])
costs = {json.loads(f.read_text())["model"]: json.loads(f.read_text()) for f in (d / "costs").glob("*.json")}

GERMAN = "mteb/germanquad-retrieval"
rows = []
for m, all_ds in q.items():
    de = all_ds.get(GERMAN)
    by_ds = {k: v for k, v in all_ds.items() if k != GERMAN}
    if not by_ds:
        continue
    hy = [r["modes"]["hybrid"]["ndcg@10"] for r in by_ds.values()]
    ve = [r["modes"]["vector"]["ndcg@10"] for r in by_ds.values()]
    rc = [r["modes"]["hybrid"]["recall@10"] for r in by_ds.values()]
    c = costs.get(m, {})
    rows.append({"model": m, "hybrid": sum(hy) / len(hy), "vector": sum(ve) / len(ve), "recall": sum(rc) / len(rc),
                 "cps": c.get("chunks_per_second"), "lat": c.get("latency_ms_p50"),
                 "bin": c.get("binary_bytes"), "db": c.get("db_bytes"), "rss": c.get("peak_rss_kb"),
                 "de": de["modes"]["hybrid"]["ndcg@10"] if de else None})


def dominates(a, b):
    if None in (a["cps"], a["bin"], b["cps"], b["bin"]):
        return False
    ge = a["hybrid"] >= b["hybrid"] and a["cps"] >= b["cps"] and a["bin"] <= b["bin"]
    gt = a["hybrid"] > b["hybrid"] or a["cps"] > b["cps"] or a["bin"] < b["bin"]
    return ge and gt


out = ["# Benchmark results", "", f"Commits: {', '.join(sorted(meta['commits']))}", "", "Datasets:"]
out += [f"- `{x}`" for x in sorted(meta["datasets"])]
out += ["", "| Model | nDCG@10 hybrid | nDCG@10 vector | Recall@10 hybrid | GermanQuAD nDCG@10 hybrid | chunks/s | p50 ms | binary MB | DB MB | RSS MB | Pareto |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
fmt = lambda v, s=1, p=1: "-" if v is None else f"{v / s:.{p}f}"
for r in sorted(rows, key=lambda r: -r["hybrid"]):
    front = "yes" if not any(dominates(o, r) for o in rows if o is not r) else ""
    out.append(f"| {r['model']} | {r['hybrid']:.4f} | {r['vector']:.4f} | {r['recall']:.4f} | {fmt(r['de'], 1, 4)} | {fmt(r['cps'])} | {fmt(r['lat'], 1, 2)} | "
               f"{fmt(r['bin'], 2**20)} | {fmt(r['db'], 2**20)} | {fmt(r['rss'], 1024)} | {front} |")
(d.parent / "RESULTS.md" if d.name == "results" else d / "RESULTS.md").write_text("\n".join(out) + "\n")
print("\n".join(out))
