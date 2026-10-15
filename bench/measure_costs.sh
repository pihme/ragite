#!/usr/bin/env bash
# CPU costs for one Model on the fixed benchmark machine (REQ-89).
# Run on the designated machine only (CI runners are too noisy), one model at a time.
# Usage: bench/measure_costs.sh <libragite.so> <out.json>
# Records binary size, DB size, chunks/s, query latency and peak RSS on SciFact.
set -euo pipefail
ext=$1 out=$2
db=$(mktemp -u /tmp/ragite-bench-XXXX.db)
/usr/bin/time -v -o /tmp/ragite-time.txt \
  python3 bench/run_quality.py --extension "$ext" --dataset mteb/scifact --db "$db" --out "$out.quality.json"
python3 - "$ext" "$db" "$out" <<'PY'
import json, os, platform, re, sys
ext, db, out = sys.argv[1:]
q = json.load(open(out + ".quality.json"))
rss = int(re.search(r"Maximum resident set size \(kbytes\): (\d+)", open("/tmp/ragite-time.txt").read()).group(1))
json.dump({"model": q["model"], "commit": q["commit"], "machine": platform.node(), "cpus": os.cpu_count(),
           "binary_bytes": os.path.getsize(ext), "db_bytes": os.path.getsize(db), "peak_rss_kb": rss,
           "chunks_per_second": q["chunks_per_second"],
           "latency_ms_p50": q["modes"]["hybrid"]["latency_ms_p50"],
           "latency_ms_p95": q["modes"]["hybrid"]["latency_ms_p95"]}, open(out, "w"), indent=1)
PY
rm -f "$db"
cat "$out"
