# Teeline

Teeline is a solver for symmetric Traveling Salesman Problem, written in Rust.

It still work in progress. Although it already has for all algorithms teached by any CS courses.
More advanced algorithms would be implemented after the structure of code and interfaces has been stabilized.


It all started from the ["In Pursuit of the Traveling Salesman"](https://www.amazon.de/Pursuit-Traveling-Salesman-Mathematics-Computation-ebook/dp/B0073X0IR2/ref=sr_1_1?_encoding=UTF8) book,
which fired my interest in Linear Programming, that is backbone of Concorde the best TSP solver;

After i finished the book, i took the [Discrete Optimization](https://coursera.org/share/1428f00fd18abc041afcf9105c02365b) course on the Coursera
to learn more about discrete optimization and TSP behind a theory covered by standard CS course.
And one of the homeworks was to implement the TSP solver, that could get also solve problem that has more than 10_000 cities,
which encouraged me to look alternative solutions.

### Preparing data

Teeline works only subset TSLIB files. It expects that cities are presented as euclidean coordinates
either in `NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION`.

```
# chmod +x download_data

./download_data.sh
```

### Exact algorithms:

TBD

Exact algorithms are guaranteed to give optimal solutions.
Although there's small catch, their running time is exponential and they cant solve problems beyond 15-20 cities.

##### branch-and-bound

exhaustive tree search with pruning;

```
./teeline branch_bound --verbose
./teeling branch_bound
```

##### Bellman-Karp-Held

dynamic algorithm

```
./teeline bellman_karp
./teeline bhk
./teeline bhk --verbose
```

### Approximate algorithms:

TBD

##### greedy nearest neighbors using KD-tree

TBD

```
./teeline nn
./teeline nn --verbose
```

##### 2-opt heuristic

```
./teeline two_opt
./teeline 2opt

./target/debug/bin 2opt -i ./data/discopt/tsp_5_1.tsp
```

##### stochastic hill climbing

Hill Climbing with random restarts

available settings:

* `verbose` - prints some debugging details onto std-out

* `epochs` - max iterations, if 0 then it would run forever

* `platoo_epochs` - max number of iterations without improvement before restarting the search

```
./teeline stochastic_hill
./teeline stochastic_hill --epochs=100
./teeling stochastic_hill --platoo_epochs=10
```

##### simulated annealing

TODO

available settings:

* `cooling_rate` - specifies how fast should the temperature decrease

* `max_temperature` - sets initial temperature, default 1000.0

* `min_temperature` - sets the final temperature, default 0.001

* `epochs` - how many iteration run before stopping the search

```
./teeline simulated_annealing
./teeline sa --verbose
./teeline sa --cooling_rate=0.1 --min_temperature=1.0
./teeline sa --max_temperature
```

##### tabu search

TODO

available options:

* `epochs` - how many iterations to run before giving up

```
./teeline tabu_search --epochs=5
```

##### genetic search

TODO

available options:

* `epochs` - limits the max number of iteration before stopping the search, default 10.000

* `mutation_probability` - sets the probability of applying random mutation for new child, default 0.001

* `n_elite` - how many individuals of each population should sent directly to next generation, default 3

```
./teeline genetic_algorithm
./teeline ga --verbose
./teeline ga --epochs = 5 --mutation_probability = 0.2
./teeline ga --n_elite = 7
```

### Usage

* build project: `cargo build`

```
# check available settings and commands
./target/debug/bin -h

# use default settings
cat ./data/tsplib/berlin52.tsp | ./target/debug/bin

# or pass files as cli argument if no extra processing is required
./target/debug/bin -i ./data/tsplib/berlin52.tsp

# use Bellman-Held-Karp algoritm as solver
# be careful, it wouldnt work for dataset bigger than 30
cat ./data/tsplib/bayg29.tsp | ./target/debug/bin bellman_karp
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

#### 2 - Opt

* Algorithms Illuminated, vol.4: https://youtu.be/dYEWqrp-mho

#### Dynamic Programming

* Bellman-Held-Karp: https://youtu.be/D8aHqaFa8GE

#### Branch and Bound

* Backtracking from Skiena's "Algorithm Design Manual": http://www.algorist.com/algorist.html
* EECS 281: S20 Lecture 21 - Backtracking and Branch & Bound (Traveling Salesperson Problem) - https://www.youtube.com/watch?v=hNs7G1b2iFY&t=5480s

* GeekForGeeks article: https://www.geeksforgeeks.org/traveling-salesman-problem-using-branch-and-bound-2/

#### Genetic algorithm
* Ch.4.1.4 - "Genetic Algorithms", AIMA, https://github.com/aimacode/aima-python/blob/ca301ea363674ec719b58f23e794998de4f623c9/search.py#L912
* Ch7.5 - Random sampling for local search, Skiena: https://books.google.de/books?id=7XUSn0IKQEgC&lpg=PR1&pg=PA251#v=onepage&q&f=false
* ch.4 "TSP", Genetic Algorithms in Java: https://learning.oreilly.com/library/view/genetic-algorithms-in/9781484203286/9781484203293_Ch04.xhtml
* comparitions of selection https://arxiv.org/ftp/arxiv/papers/1203/1203.3099.pdf
* ordered crossover: http://www.dmi.unict.it/mpavone/nc-cs/materiale/moscato89.pdf
* comparition of crossover methods: http://www.iro.umontreal.ca/~dift6751/ga_tsp_tr.pdf


