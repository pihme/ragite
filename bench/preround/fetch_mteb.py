#!/usr/bin/env python3
"""Preround: read published MTEB retrieval scores for the 11 candidates.

No model is run. The script reads the result files behind the public MTEB
leaderboard from https://github.com/embeddings-benchmark/results and writes
bench/preround/mteb_scores.md (a table with source, results commit, and date).

    python3 bench/preround/fetch_mteb.py   # stdlib only; ~30 GitHub API calls

Set GITHUB_TOKEN to raise the API rate limit. The maintainer picks the top 4-5
from the table by hand and records the pick in bench/README.md.
"""
import datetime
import json
import os
import pathlib
import tomllib
import urllib.error
import urllib.request

REPO = "embeddings-benchmark/results"
TASKS = ["SciFact", "NFCorpus", "FiQA2018"]
ROOT = pathlib.Path(__file__).resolve().parents[2]


def get(url: str):
    req = urllib.request.Request(url, headers={"User-Agent": "ragite-bench"})
    token = os.environ.get("GITHUB_TOKEN")
    if token and "api.github.com" in url:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def score(model_dir: str, task: str, sha: str):
    """Return (ndcg@10, recall@10, revision) for the newest revision that has the task."""
    try:
        revs = [x["name"] for x in get(f"https://api.github.com/repos/{REPO}/contents/results/{model_dir}?ref={sha}") if x["type"] == "dir"]
    except urllib.error.HTTPError:
        return None
    # Prefer real revisions over "external" (scores reported by the model author).
    for rev in sorted(revs, key=lambda r: r == "external"):
        url = f"https://raw.githubusercontent.com/{REPO}/{sha}/results/{model_dir}/{rev}/{task}.json"
        try:
            data = get(url)
        except urllib.error.HTTPError:
            continue
        splits = data.get("scores", {})
        split = splits.get("test") or splits.get("dev") or next(iter(splits.values()), None)
        if not split:
            continue
        s = split[0]
        return s.get("ndcg_at_10"), s.get("recall_at_10"), rev
    return None


def main() -> None:
    sha = get(f"https://api.github.com/repos/{REPO}/commits/main")["sha"]
    models = tomllib.loads((ROOT / "bench/candidates.toml").read_text())["model"]
    lines = [
        "# MTEB preround scores",
        "",
        f"Source: <https://github.com/{REPO}> at commit `{sha}` (the data behind the MTEB leaderboard).",
        f"Fetched: {datetime.date.today().isoformat()} with `bench/preround/fetch_mteb.py`.",
        "Metric: nDCG@10 on the test split (dev if there is no test split). `-` means no published result.",
        "Revision `external` means the score was reported by the model author, not rerun by MTEB.",
        "",
        "| Model | " + " | ".join(TASKS) + " | Mean | Revision |",
        "| --- | " + " | ".join("---" for _ in TASKS) + " | --- | --- |",
    ]
    for m in models:
        model_dir = m["hf"].replace("/", "__")
        cells, en, rev_used = [], [], set()
        for t in TASKS:
            r = score(model_dir, t, sha)
            if r and r[0] is not None:
                cells.append(f"{r[0]:.4f}")
                rev_used.add(r[2][:12])
                en.append(r[0])
            else:
                cells.append("-")
        mean = f"{sum(en) / len(en):.4f}" if len(en) == 3 else "-"
        lines.append(f"| {m['name']} | " + " | ".join(cells) + f" | {mean} | {', '.join(sorted(rev_used)) or '-'} |")
    out = ROOT / "bench/preround/mteb_scores.md"
    out.write_text("\n".join(lines) + "\n")
    print(out.read_text())


if __name__ == "__main__":
    main()
