# MTEB preround scores

Source: <https://github.com/embeddings-benchmark/results> at commit `f086caaab6cefaa9a62e708e38837366ba208dff` (the data behind the MTEB leaderboard).
Fetched: 2026-10-15 with `bench/preround/fetch_mteb.py`.
Metric: nDCG@10 on the test split (dev if there is no test split). `-` means no published result.
Revision `external` means the score was reported by the model author, not rerun by MTEB.

| Model | SciFact | NFCorpus | FiQA2018 | Mean | Revision |
| --- | --- | --- | --- | --- | --- |
| potion-base-8M | 0.5064 | 0.2427 | 0.1662 | 0.3051 | dcbec7aa2d52, external |
| potion-base-32M | - | - | 0.1798 | - | fc914ed07732 |
| potion-retrieval-32M | - | - | 0.1876 | - | 3c52220caac0 |
| all-MiniLM-L6-v2 | 0.6451 | 0.3159 | 0.3687 | 0.4432 | 8b3219a92973 |
| bge-small-en-v1.5 | 0.7127 | 0.3426 | 0.4034 | 0.4863 | 5c38ec7c405e, external |
| e5-small-v2 | 0.6885 | 0.3245 | 0.3743 | 0.4625 | dca8b1a9dae0, external |
| nomic-embed-text-v1.5 | 0.7028 | 0.3467 | 0.3746 | 0.4747 | b0753ae76394, external |
| potion-multilingual-128M | 0.4514 | 0.1731 | 0.1036 | 0.2427 | 38ebd7f10f71 |
| multilingual-e5-small | 0.6758 | 0.3110 | 0.3313 | 0.4394 | fd1525a9fd15 |
| paraphrase-multilingual-MiniLM-L12-v2 | 0.4849 | 0.2341 | 0.2033 | 0.3074 | bf3bf13ab40c |
| gte-multilingual-base | 0.7344 | 0.3639 | 0.4501 | 0.5161 | 7fc06782350c, external |
