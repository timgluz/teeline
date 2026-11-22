# Algorithm candidates

A shared research backlog for solvers we might add to Teeline. Check this table before
researching a new algorithm, and update the row when a decision is made — the point is to
avoid investigating the same idea twice.

This is not a roadmap: `proposed` means worth a look, not scheduled work.

## States

| State | Meaning |
| --- | --- |
| `proposed` | Worth investigating; no decision yet. |
| `accepted` | Approved for implementation; link the issue/PR in the description. |
| `declined` | Investigated and rejected; the description records why. |
| `hold` | Not now; the description records what would unblock it. |

## Candidates

| algorithm_id | algo family | state | short description | references |
| --- | --- | --- | --- | --- |
| `attention-model` | learned | proposed | Kool et al. attention encoder–decoder. 2.8 MB TSP100 checkpoint, MIT; one greedy rollout; the simplest first learned-family candidate. Needs ONNX + a Rust runtime (`tract`) or a Python sidecar. | [paper](https://arxiv.org/abs/1803.08475), [repo](https://github.com/wouterkool/attention-learn-to-route) |
| `pomo` | learned | proposed | Multi-start policy optimization. ~1% TSP100 greedy, ~0.14% with 8× augmentation; 15 MB MIT checkpoint. Stronger than the attention model, but needs multi-start decoding; consider after `attention-model`. | [paper](https://arxiv.org/abs/2010.16011), [repo](https://github.com/yd-kwon/POMO) |
| `rlor-tsp` | learned | hold | Hugging Face-hosted PPO attention model (2.9 MB, MIT), trained on TSP50; its demo Space is broken. Useful as a quick HF plumbing proof, not as a final family member. | [HF](https://huggingface.co/cpwan/RLOR-TSP), [paper](https://arxiv.org/abs/2303.13117) |
| `floydnet` | learned | hold | Hugging Face checkpoint (851 MB, MIT) for Euclidean TSP; >96% optimality at N≤100, O(N³). Too heavy for WASM; API/GPU-only if ever used. | [HF](https://huggingface.co/ocxlabs/FloydNet_TSP_euc), [paper](https://arxiv.org/abs/2601.19094) |
| `difusco` | learned | hold | Diffusion-based edge-heatmap solver. Strong TSP100 quality but heavy iterative inference; MIT checkpoints via Google Drive. API-only. | [repo](https://github.com/yikai-wu/DIFUSCO), [paper](https://arxiv.org/abs/2302.08224) |
| `fast-t2t` | learned | hold | Training-to-testing diffusion solver. Strong TSPLIB quality and MIT licensed, but heavy; checkpoints via the project link. | [repo](https://github.com/Thinklab-SJTU/Fast-T2T), [paper](https://arxiv.org/abs/2406.19705) |
| `lehd` | learned | hold | Light encoder / heavy decoder. Strong large-scale generalization (TSP1000 greedy ~3.2%, RRC ~1.2%); pretrained availability unclear; heavy inference. | [repo](https://github.com/CIAM-Group/NCO_code/tree/main/single_objective/LEHD), [paper](https://arxiv.org/abs/2310.07985) |
| `glop` | learned | hold | Hierarchical partition + local construction; scales to TSP100k. No license file found and heavy PyTorch Geometric dependencies. | [repo](https://github.com/henry-yeh/GLOP), [paper](https://arxiv.org/abs/2312.08224) |
| `neurolkh` | learned/local | hold | Learned edge scores and node penalties guide LKH. Near-LKH quality, but the repo has no license and it needs the LKH C code. Do not integrate until licensed. | [repo](https://github.com/liangxinedu/NeuroLKH), [paper](https://arxiv.org/abs/2110.07983) |
| `vitsp` | learned (VLM) | hold | A pretrained vision-language model selects subproblems for LKH/Concorde; 0.24% gap on 1k–88k TSPLIB. Proprietary API + exact solver; a research curiosity, not locally benchmarkable. | [paper](https://mlanthology.org/iclr/2026/yin2026iclr-vitsp/) |
| `astar-1tree` | exact | declined | A*/best-first branch-and-bound over edge decisions with the Held–Karp 1-tree bound. Rejected: per-node subgradient tuning, brittle forced-edge handling, and the 14–17 city exact benchmark would not show its advantage. | [Held–Karp 1970](https://doi.org/10.1287/opre.18.6.1138), [Volgenant–Jonker 1982](https://doi.org/10.1016/0377-2217(82)90015-7), [Valenzuela–Jones 1997](https://homes.di.unimi.it/righini/Didattica/OttimizzazioneDiscreta/MaterialeOD/1997%20Valenzuela%20Jones%20-%20HK%20lower%20bound.pdf) |
| `held-karp-bound` | exact (diagnostic) | hold | Standalone 1-tree Lagrangian lower bound as a gap estimator: the same insight as `astar-1tree` without the search tree. No solver entry; revisit as a benchmark metric. | [Held–Karp 1971](https://doi.org/10.1007/BF01584070), [Valenzuela–Jones 1997](https://homes.di.unimi.it/righini/Didattica/OttimizzazioneDiscreta/MaterialeOD/1997%20Valenzuela%20Jones%20-%20HK%20lower%20bound.pdf) |
| `cpsat-tsp` | exact | proposed | OR-Tools CP-SAT circuit/MTZ formulation as a third exact member. Apache-2.0 and strong on small/medium instances, but needs a Python/C++ sidecar or FFI and an ADR for the dependency. | [OR-Tools](https://github.com/google/or-tools), [CP-SAT](https://developers.google.com/optimization/cp/cp_solver) |
| `concorde` | exact | declined | Gold-standard branch-and-cut; exact to thousands of cities. Rejected: external C code, license/distribution constraints, not pure Rust. | [Concorde](https://www.math.uwaterloo.ca/tsp/concorde.html) |
| `milp-mtz` | exact | hold | Pure-Rust MILP (for example `microlp`) with MTZ/DFJ. Dependency-free but likely weak beyond small n; benchmark against BHK and `branch_bound` before committing. | [good_lp](https://github.com/rust-or/good_lp), [microlp](https://github.com/rust-or/microlp) |
| `insertion-heuristics` | constructive | proposed | Nearest/farthest/cheapest insertion: zero tuning, deterministic, O(n²); easy constructive-family additions. Farthest insertion is the most distinct from `nn`. | [Rosenkrantz et al. 1977](https://doi.org/10.1137/0206041) |
| `double-tree` | approximation | hold | MST double-tree 2-approximation; dominated by the existing Christofides 1.5× bound. Only useful if the approximation family is made explicit. | [Christofides](https://en.wikipedia.org/wiki/Christofides_algorithm) |
| `guided-local-search` | meta | proposed | Penalizes frequently used edges to escape local optima. Strong TSP results but has penalty/λ parameters; good defaults would need calibration. Benchmark against tabu and SA. | [GLS](https://en.wikipedia.org/wiki/Guided_local_search) |
| `iterated-local-search` | meta | declined | Already effectively covered by `lk` (2-opt + double-bridge kicks); a separate ILS would duplicate it. | [ILS](https://en.wikipedia.org/wiki/Iterated_local_search) |
| `variable-neighborhood-search` | meta | hold | Systematic neighborhood changes; robust and parameter-light, but the meta family already has nine members. | [VNS](https://en.wikipedia.org/wiki/Variable_neighborhood_search) |
| `lkh-3` | local | declined | The reference LKH implementation. Strong, but external C, license terms, and a large integration; `lk` remains the pure-Rust representative. | [LKH-3](http://akira.ruc.dk/~keld/research/LKH-3/) |
| `portfolio` | strategy | hold | Run a small portfolio of existing solvers under one budget and return the best. Strong, but not a published algorithm and needs a fairness/budget ADR. | [Algorithm selection](https://en.wikipedia.org/wiki/Algorithm_selection) |
| `llm-end-to-end` | learned | declined | Ask an LLM to emit a tour directly: expensive, slow, non-reproducible and poor TSP quality. ViTSP's decomposition use is the interesting version. | [ViTSP](https://mlanthology.org/iclr/2026/yin2026iclr-vitsp/) |

## Notes

- **`learned` is not an existing Teeline family.** Adding it requires a constitution/ADR
  update; the table lists it because the pretrained-model route is the only genuinely new
  family currently on the table.
- **`approximation` and `strategy` are candidate families**, not current site categories.
- **Keep IDs stable and kebab-case.** They are the handle for issues, PRs and future
  benchmark metadata.
- **Do not delete declined rows.** The record of why an idea was rejected is the reason this
  file exists.
