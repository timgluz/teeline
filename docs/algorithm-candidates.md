# Algorithm candidates

A shared research backlog for solvers we might add to Teeline. Check this table before
researching a new algorithm, and update the row when a decision is made — the point is to
avoid investigating the same idea twice.

This is not a roadmap: `proposed` means worth a look, not scheduled work.

Jump to: [accepted](#accepted) · [proposed](#proposed) · [hold](#hold) · [declined](#declined).

## States

| State | Meaning |
| --- | --- |
| `accepted` | Approved for implementation; `comments` names the issue/PR. |
| `proposed` | Worth investigating; no decision yet. |
| `hold` | Not now; `comments` records what would unblock it. |
| `declined` | Investigated and rejected; `comments` records why. |

## Candidates

### Accepted

| algorithm_id | algo family | state | short description | references | comments |
| --- | --- | --- | --- | --- | --- |
| `spectral_fiedler` | constructive | accepted | Spectral constructive solver: kNN affinity graph, normalized Laplacian eigenvectors, polar-angle ordering. | [#646](https://github.com/timgluz/teeline/issues/646), [Fiedler vector](https://en.wikipedia.org/wiki/Algebraic_connectivity) | Accepted 2026-11 in #646; zero-dependency plan with aliases `spectral`/`fiedler`. Define behavior for disconnected graphs and asymmetric EXPLICIT inputs before implementation. |

### Proposed

| algorithm_id | algo family | state | short description | references | comments |
| --- | --- | --- | --- | --- | --- |
| `attention-model` | learned | proposed | Kool et al. attention encoder–decoder; autoregressive construction with one greedy rollout. | [paper](https://arxiv.org/abs/1803.08475), [repo](https://github.com/wouterkool/attention-learn-to-route) | Simplest first learned-family candidate: 2.8 MB TSP100 checkpoint, MIT. Gate on ONNX + `tract` (or a Python sidecar) before committing to the family. |
| `pomo` | learned | proposed | Multi-start policy optimization: many greedy rollouts from different starts, keep the best. | [paper](https://arxiv.org/abs/2010.16011), [repo](https://github.com/yd-kwon/POMO) | Stronger than `attention-model` (~1% TSP100 greedy, ~0.14% with 8× augmentation) but multi-start decoding is extra work. Try after the first learned baseline. |
| `cpsat-tsp` | exact | proposed | OR-Tools CP-SAT circuit/MTZ formulation. | [OR-Tools](https://github.com/google/or-tools), [CP-SAT](https://developers.google.com/optimization/cp/cp_solver) | Best non-ML exact addition, but needs a Python/C++ sidecar or FFI and an ADR for the dependency. |
| `insertion-heuristics` | constructive | proposed | Nearest/farthest/cheapest insertion constructors. | [Rosenkrantz et al. 1977](https://doi.org/10.1137/0206041) | Zero tuning, deterministic, O(n²); farthest insertion is the most distinct from `nn`. |
| `dynasearch` | local | proposed | Dynamic-programming local search: applies the jointly optimal batch of independent 2-opt reversals in one pass. | [Potts & van de Velde 1995](https://research.utwente.nl/en/publications/dynasearch-interative-local-improvement-by-dynamic-programming-pa), [Congram et al. 2002](https://doi.org/10.1287/ijoc.14.1.52.7712) | Searches an exponential-size neighborhood in polynomial time, unlike single-move 2-opt/3-opt. Pure Rust, ~200 LOC; O(n²) per pass, best at n≤2000. |
| `balas-simonetti-dp` | local/exact | proposed | Sliding-window bitmask DP: finds the best tour where each city moves at most k positions. | [Balas & Simonetti 2001](https://doi.org/10.1287/ijoc.13.1.56.9748) | Deterministic and linear-time in n (exponential in k); reuses BHK-style bitmask DP on any instance size and works on EXPLICIT matrices. k=6–10 is practical, but the state definition is fiddly. |
| `guided-local-search` | meta | proposed | Penalize frequently used edges to escape local optima. | [GLS](https://en.wikipedia.org/wiki/Guided_local_search) | Strong TSP results but penalty/λ parameters need calibration; benchmark before adding another meta solver. |
| `popmusic` | decomposition | proposed | Re-optimize short tour stretches (R consecutive cities plus nearby cities) with fixed endpoints until no stretch improves. | [Taillard & Helsgaun 2019](https://doi.org/10.1016/j.ejor.2018.06.039), [Taillard 2019](https://arodes.hes-so.ch/nanna/record/8174/files/Published%20version.pdf) | Proposed independently by both reviewers, so the idea has legs. New family: pairs `bhk`/`3opt`/`lk` as sub-solvers; pure Rust, no O(n²) full-matrix requirement. Needs an R parameter and a family ADR. |

### Hold

| algorithm_id | algo family | state | short description | references | comments |
| --- | --- | --- | --- | --- | --- |
| `bq-nco` | learned | hold | Bisimulation-quotiented MDP policy; each decision solves the remaining tail subproblem. | [paper](https://arxiv.org/abs/2301.03313), [repo](https://github.com/naver/bq-nco) | Strong out-of-distribution generalization with one greedy rollout, but the BQ-MDP decoder runs the model at every step, so inference is pricier. License needs checking (repo reports NOASSERTION). |
| `rlor-tsp` | learned | hold | PPO-trained attention model hosted on Hugging Face. | [HF](https://huggingface.co/cpwan/RLOR-TSP), [paper](https://arxiv.org/abs/2303.13117) | TSP50-only and its demo Space is broken; useful only as a quick HF plumbing proof, not a final family member. |
| `dimes` | learned | hold | Differentiable meta-solver: continuous solution distribution + MAML initialization + active search. | [paper](https://arxiv.org/abs/2210.04123), [repo](https://github.com/DIMESTeam/DIMES) | Scales to 10k nodes but heavy (PyTorch Geometric, custom CUDA sampler); no lightweight/WASM path. |
| `maskco` | learned | hold | Masked-generation self-supervised solver with mask-and-reconstruct inference. | [paper](https://openreview.net/forum?id=psUjNnLhl9) | Current SOTA on TSP/CVRP/MIS, but requires training and no ready checkpoint was found. |
| `floydnet` | learned | hold | Pairwise relational network for Euclidean TSP; Hugging Face checkpoint. | [HF](https://huggingface.co/ocxlabs/FloydNet_TSP_euc), [paper](https://arxiv.org/abs/2601.19094) | 851 MB and O(N³); API/GPU-only, not for WASM. |
| `difusco` | learned | hold | Diffusion over edge heatmaps; decode a tour from the denoised adjacency. | [repo](https://github.com/yikai-wu/DIFUSCO), [paper](https://arxiv.org/abs/2302.08224) | Strong TSP100 quality but heavy iterative inference; MIT checkpoints via Google Drive. API-only. |
| `fast-t2t` | learned | hold | Training-to-testing diffusion solver with consistency acceleration. | [repo](https://github.com/Thinklab-SJTU/Fast-T2T), [paper](https://arxiv.org/abs/2406.19705) | Strong TSPLIB quality and MIT, but heavy; checkpoints via the project link. |
| `lehd` | learned | hold | Light encoder / heavy decoder built for large-scale generalization. | [repo](https://github.com/CIAM-Group/NCO_code/tree/main/single_objective/LEHD), [paper](https://arxiv.org/abs/2310.07985) | TSP1000 greedy ~3.2%, RRC ~1.2%; pretrained availability unclear and inference is heavy. |
| `glop` | learned | hold | Hierarchical partition + local construction for very large TSP. | [repo](https://github.com/henry-yeh/GLOP), [paper](https://arxiv.org/abs/2312.08224) | Scales to TSP100k, but no license file found and heavy PyTorch Geometric dependencies. |
| `goal` | learned | hold | Generalist multi-task CO agent with a shared backbone. | [ICLR 2025 slides](https://www.iclr.cc/media/iclr-2025/Slides/27692.pdf) | Not TSP-specific; a later direction if a generalist model is ever wanted. |
| `neurolkh` | learned/local | hold | Learned edge scores and node penalties guiding LKH. | [repo](https://github.com/liangxinedu/NeuroLKH), [paper](https://arxiv.org/abs/2110.07983) | Repo has no license and requires the LKH C code; do not integrate until licensed. |
| `vitsp` | learned (VLM) | hold | Pretrained vision-language model selects subproblems; an exact solver finishes them. | [paper](https://mlanthology.org/iclr/2026/yin2026iclr-vitsp/) | Proprietary VLM API + exact solver; 0.24% on 1k–88k TSPLIB but not locally benchmarkable. |
| `milp-mtz` | exact | hold | Pure-Rust MILP (for example `microlp`) with MTZ/DFJ. | [good_lp](https://github.com/rust-or/good_lp), [microlp](https://github.com/rust-or/microlp) | Dependency-free but likely weak beyond small n; benchmark against BHK and `branch_bound` first. |
| `held-karp-bound` | exact (diagnostic) | hold | Standalone 1-tree Lagrangian lower bound. | [Held–Karp 1971](https://doi.org/10.1007/BF01584070), [Valenzuela–Jones 1997](https://homes.di.unimi.it/righini/Didattica/OttimizzazioneDiscreta/MaterialeOD/1997%20Valenzuela%20Jones%20-%20HK%20lower%20bound.pdf) | Same insight as `astar-1tree` without the search tree; revisit as a benchmark gap metric, not a solver. |
| `gpx-lk` | meta/local | hold | Deterministic partition crossover (GPX/GPX2) over LK-optimized parent tours. | [Whitley et al. 2009](https://doi.org/10.1145/1569901.1569987), [Tinós et al. 2020](https://doi.org/10.1162/evco_a_00254) | O(n), no population tuning, but conceptually close to `eax`; benchmark GPX against `eax` first. GPX2's extra fusion steps add complexity, so start with plain GPX. |
| `softassign-sinkhorn` | relaxation | hold | Relax the tour permutation matrix into the doubly stochastic Birkhoff polytope via Sinkhorn normalization and annealing, then round. | [Gold & Rangarajan 1996](https://proceedings.neurips.cc/paper_files/paper/1995/file/b1563a78ec59337587f6ab6397699afc-Paper.pdf), [Rangarajan et al. 1996](https://doi.org/10.1162/neco.1996.8.5.1041) | Genuinely orthogonal: relaxes permutations directly and works on EXPLICIT matrices without an embedding. O(n²) memory/step, annealing-sensitive, and rounded matrices still need subtour patching; quality trails 2opt/lk. Needs a relaxation-family ADR. |
| `portfolio` | strategy | hold | Run a small portfolio under one budget and return the best. | [Algorithm selection](https://en.wikipedia.org/wiki/Algorithm_selection) | Not a published algorithm; needs a fairness/budget ADR before it belongs in the benchmark. |
| `si-tsp` | matheuristic | hold | Spatially informed MIP: Voronoi-based variable pruning before a MILP. | [paper](https://www.tandfonline.com/doi/full/10.1080/00330124.2025.2565474) | Niche hybrid exact/heuristic; no obvious pure-Rust path. |

### Declined

| algorithm_id | algo family | state | short description | references | comments |
| --- | --- | --- | --- | --- | --- |
| `llm-end-to-end` | learned | declined | Ask an LLM to emit a tour directly. | [ViTSP](https://mlanthology.org/iclr/2026/yin2026iclr-vitsp/) | Expensive, slow, non-reproducible and poor TSP quality; ViTSP's decomposition use is the interesting version. |
| `concorde` | exact | declined | Branch-and-cut reference solver. | [Concorde](https://www.math.uwaterloo.ca/tsp/concorde.html) | External C code, license/distribution constraints, not pure Rust. |
| `astar-1tree` | exact | declined | A*/best-first B&B over edge decisions with the Held–Karp 1-tree bound. | [Held–Karp 1970](https://doi.org/10.1287/opre.18.6.1138), [Volgenant–Jonker 1982](https://doi.org/10.1016/0377-2217(82)90015-7), [Valenzuela–Jones 1997](https://homes.di.unimi.it/righini/Didattica/OttimizzazioneDiscreta/MaterialeOD/1997%20Valenzuela%20Jones%20-%20HK%20lower%20bound.pdf) | Rejected 2026-11: per-node subgradient tuning and brittle forced-edge handling, and the 14–17 city exact benchmark would not show its advantage. |
| `lkh-3` | local | declined | Reference Lin–Kernighan–Helsgaun implementation. | [LKH-3](http://akira.ruc.dk/~keld/research/LKH-3/) | External C + license terms; `lk` remains the pure-Rust representative. |

## Notes

- **`learned` is not an existing Teeline family.** Adding it requires a constitution/ADR
  update; the table lists it because the pretrained-model route is the only genuinely new
  family currently on the table.
- **`decomposition`, `relaxation`, `strategy` and `matheuristic` are candidate families**,
  not current site categories.
- **Keep IDs stable and kebab-case.** They are the handle for issues, PRs and future
  benchmark metadata.
- **Do not delete declined rows.** The record of why an idea was rejected is the reason this
  file exists.
- **Further reading:** the [NCO survey](https://arxiv.org/abs/2406.00415) and the
  [NCO tutorial](https://doi.org/10.1016/j.cor.2025.107102) cover the learning-to-optimize
  landscape; start there before adding another neural candidate.
