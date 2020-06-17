# Teeline

Teeline is collection of algorithms for the Traveling Salesman Problem.
It could be used as command-line utility or as Rust package;

It currently implements:

* greedy nearest neighbors using KD-tree
* 2-opt
* stochastic hill climbing with random restarts
* simulated annealing
* tabu search


### Usage

* build project: `cargo build`

```
# check available settings and commands
./target/debug/bin -h

# use default settings
cat ./data/tsp_51_1 | ./target/debug/bin

# use Bellman-Held-Karp algoritm as solver
# be careful, it wouldnt work for dataset bigger than 30
cat ./data/tsp_5_1 | ./target/debug/bin --solver=bellman_karp
```

* compile runnable binary:

```
cargo build --release
```


*Visualizing solution*


generate solution file

```
cat ./data/tsp_51_1 | ./target/debug/bin > solution51.txt
```

upload data file and solution file to:
https://discreteoptimization.github.io/vis/tsp/ to visualize solution;
