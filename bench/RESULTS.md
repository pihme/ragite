# Benchmark results

Commits: e3ddacbf9b4bc7f2e4a4f2ea7e90114fb7039300

Datasets:
- `{"id": "mteb/fiqa", "revision": "main", "skipped_empty_docs": 38}`
- `{"id": "mteb/nfcorpus", "revision": "main", "skipped_empty_docs": 0}`
- `{"id": "mteb/scifact", "revision": "main", "skipped_empty_docs": 0}`

| Model | nDCG@10 hybrid | nDCG@10 vector | Recall@10 hybrid | chunks/s | p50 ms | binary MB | DB MB | RSS MB | Pareto |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7 | 0.3759 | 0.3740 | 0.3864 | 4325.9 | 19.79 | 131.2 | 70.2 | 452.7 | yes |
| potion-base-32M@1e5a03f8eeb2c98b928fbbd846f22f816360919f | 0.3502 | 0.3439 | 0.3601 | 4303.7 | 19.35 | 131.2 | 70.2 | 452.6 | yes |
| potion-base-8M@bf8b056651a2c21b8d2565580b8569da283cab23 | 0.3309 | 0.3211 | 0.3413 | 5379.5 | 9.14 | 36.0 | 47.1 | 257.1 | yes |
| potion-multilingual-128M@73908c3438cf03b6a01bcb9611d62b23d0726f08 | 0.2767 | 0.2613 | 0.2928 | 2794.6 | 10.46 | 512.9 | 47.1 | 1774.6 |  |
