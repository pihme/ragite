#!/usr/bin/env python3
"""Per-model parity check before the full run (REQ-92).

Embeds the sentences in bench/parity/sentences.txt through the extension
(`ragite_embed_model`) and through the upstream reference (model2vec for
Model2Vec models, sentence-transformers/PyTorch for transformers), with the
model's prefixes, and fails if any cosine is below 0.99. A failure means a
bug in loading or quantisation, not a weak Model.

    python3 bench/parity/check_parity.py --extension libragite.so --hf BAAI/bge-small-en-v1.5 --kind transformer
"""
import argparse
import pathlib
import sqlite3
import struct
import sys

import numpy as np


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--extension", required=True)
    ap.add_argument("--hf", required=True)
    ap.add_argument("--revision")
    ap.add_argument("--kind", choices=["model2vec", "transformer"], required=True)
    ap.add_argument("--passage-prefix", default="")
    ap.add_argument("--min", type=float, default=0.99)
    a = ap.parse_args()
    sentences = [s for s in (pathlib.Path(__file__).parent / "sentences.txt").read_text().splitlines() if s.strip()]

    if a.kind == "model2vec":
        from model2vec import StaticModel
        ref = StaticModel.from_pretrained(a.hf).encode([a.passage_prefix + s for s in sentences])
    else:
        from sentence_transformers import SentenceTransformer
        m = SentenceTransformer(a.hf, revision=a.revision, device="cpu", trust_remote_code=True)
        ref = m.encode([a.passage_prefix + s for s in sentences], normalize_embeddings=True)

    con = sqlite3.connect(":memory:")
    con.enable_load_extension(True)
    con.load_extension(a.extension)
    mid = con.execute("SELECT technical_id FROM ragite_default_model()").fetchone()[0]
    worst = 1.0
    for s, r in zip(sentences, ref):
        blob = con.execute("SELECT ragite_embed_model(?, ?, 'passage')", (mid, s)).fetchone()[0]
        v = np.array(struct.unpack(f"<{len(blob)//4}f", blob))
        c = float(v @ r / (np.linalg.norm(v) * np.linalg.norm(r)))
        worst = min(worst, c)
    print(f"{mid}: {len(sentences)} sentences, min cosine {worst:.5f}")
    sys.exit(0 if worst >= a.min else 1)


if __name__ == "__main__":
    main()
