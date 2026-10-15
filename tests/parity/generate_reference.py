#!/usr/bin/env python3
"""Generate the parity reference vectors for the bundled default Model.

The reference is the upstream Python implementation of the Model. For a
Model2Vec model (potion-base-8M) that is the `model2vec` package, which is
NumPy-only and needs no PyTorch. For a transformer candidate the benchmark
harness uses sentence-transformers (PyTorch) instead, see bench/README.md.

Usage (cheap, ~100 MB of wheels, runs on a laptop):

    python3 -m venv .venv && .venv/bin/pip install 'model2vec==0.6.0'
    .venv/bin/python tests/parity/generate_reference.py

Commit the resulting tests/parity/reference.json.
"""
import json
import pathlib

import model2vec

ROOT = pathlib.Path(__file__).resolve().parents[2]
MODEL_DIR = ROOT / "models" / "potion-base-8M"
TECHNICAL_ID = "potion-base-8M@bf8b056651a2c21b8d2565580b8569da283cab23"


def main() -> None:
    sentences = (ROOT / "tests/parity/sentences.txt").read_text(encoding="utf-8").split("\n")
    sentences = [s for s in sentences if s != ""]
    model = model2vec.StaticModel.from_pretrained(str(MODEL_DIR))
    vectors = model.encode(sentences)
    out = {
        "model": TECHNICAL_ID,
        "reference": f"model2vec {model2vec.__version__} (Python)",
        "min_cosine": 0.99,
        "items": [
            {"text": s, "vector": [round(float(x), 7) for x in v]}
            for s, v in zip(sentences, vectors)
        ],
    }
    path = ROOT / "tests/parity/reference.json"
    path.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {len(sentences)} vectors of dim {len(vectors[0])} to {path}")


if __name__ == "__main__":
    main()
