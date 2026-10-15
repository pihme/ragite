# Own question set (frozen)

30-50 questions over real documents, in English.

- `docs/*.json`: real PDFs converted with Docling (`docling --to json`), so Chunks are cut the way they are in production. Record the Docling version and source PDF of each file in `SOURCES.md`.
- `own.jsonl`: one row per question, `{"id": "q01", "lang": "en", "question": "...", "relevant": ["file.json", ...]}`. `relevant` lists the document files that answer it.

Freeze rule: once a run is published, the file is not edited; a change gets a new file name (`own-v2.jsonl`). `run_quality.py` records its sha256 in every result.

Status: empty. Fill after the Docling heading fix from PR #11 lands (benchmark depends on it).
