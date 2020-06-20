# Teeline

Teeline is collection of algorithms for the Traveling Salesman Problem.
It could be used as command-line utility or as Rust package;

It currently implements:

* greedy nearest neighbors using KD-tree
* 2-opt
* stochastic hill climbing with random restarts
* simulated annealing
* tabu search
* genetic search

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

## Resources

#### Genetic algorithm

* comparitions of selection https://arxiv.org/ftp/arxiv/papers/1203/1203.3099.pdf
* ordered crossover: http://www.dmi.unict.it/mpavone/nc-cs/materiale/moscato89.pdf
* Java implementation: https://learning.oreilly.com/library/view/genetic-algorithms-in/9781484203286/9781484203293_Ch04.xhtml
* comparition of crossover methods: http://www.iro.umontreal.ca/~dift6751/ga_tsp_tr.pdf
* Ch7.5 - Random sampling for local search, Skiena: https://books.google.de/books?id=7XUSn0IKQEgC&lpg=PR1&pg=PA251#v=onepage&q&f=false


