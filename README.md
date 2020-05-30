# Teeline

Teeline is collection of algorithms for the Traveling Salesman Problem.
It could be used as command-line utility or as Rust package;

It currently implements:

* greedy nearest neighbors using KD-tree
* 2-opt


### Usage

* build project: `cargo build`

* using as CLI: `cat ./data/tsp_51_1 | ./target/debug/bin`


*Visualizing solution*


generate solution file

```
cat ./data/tsp_51_1 | ./target/debug/bin > solution51.txt
```

upload data file and solution file to:
https://discreteoptimization.github.io/vis/tsp/ to visualize solution;