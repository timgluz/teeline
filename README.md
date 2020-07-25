# Teeline

Teeline is a solver for the symmetric Traveling Salesman Problem, written in Rust.

*The Traveling Salesman Problem (TSP) is the searchfor  a  minimum  cost  Hamiltonian  circuit  connecting  aset  of  locations.*[source](http://www.optimization-online.org/DB_FILE/2017/12/6370.pdf)

It still work in progress. Although it already has for all algorithms teached by any CS courses.
More advanced algorithms would be implemented after the structure of code and interfaces has been stabilized.

###### Backstory

It all started from the ["In Pursuit of the Traveling Salesman"](https://www.amazon.de/Pursuit-Traveling-Salesman-Mathematics-Computation-ebook/dp/B0073X0IR2/ref=sr_1_1?_encoding=UTF8) book. It is an fantastic book, it covers history of the Salesman problem and the big idea behind the Concorde solver.
I was sincerely suprised that the Linear Programming worked so well for this problem and can provide exact solutions for very big problems.


After i finished the book, i took the [Discrete Optimization](https://coursera.org/share/1428f00fd18abc041afcf9105c02365b) course on the Coursera.
So i could learn more about discrete optimization and a theory behind the Concorde solver.

One of the homeworks also asked to build the TSP solver, that could get also solve problem that has more than 10_000 cities,
which encouraged me to experiment alternative solutions and give me good chance to learn Rust.

### Getting started

* install rust: [](https://www.rust-lang.org/tools/install)

* compile binary: `cargo build --release`

* copy binary to your workfolder: `cp ./target/release/bin teeline`

* test the binary: `./teeline -h`


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


### Using dev version

```
# build project
cargo build

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

### Preparing data

Teeline works only subset TSPLIB files - it expects that cities are presented as euclidean coordinates
either in `NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION`.

One can use the [convert2tsplib](https://github.com/timgluz/teeline/blob/master/convert2tsplib.py) to convert list of euclidean coordinates to TSPLIB file;


```
# chmod +x download_data

./download_data.sh
```

## Exact algorithms:

*In computer science and operations research, exact algorithms are algorithms that always solve an optimization problem to optimality. *[wiki](https://en.wikipedia.org/wiki/Exact_algorithm)

Exact algorithms are guaranteed to give optimal solutions.
Although there's a small catch - a running time is exponential and they can solve only very small problems, 
because their running time complexity is either factorial or exponential.

#### branch-and-bound


A **branch-and-bound** algorithm consists of a systematic enumeration of candidate solutions by means of state space search: the set of candidate solutions is thought of as forming a rooted tree with the full set at the root. 

The algorithm explores branches of this tree(*branching*), which represent subsets of the solution set. Before enumerating the candidate solutions of a branch, the branch is checked against *upper* and *lower* estimated bounds on the optimal solution (*bounding*), and is discarded if it cannot produce a better solution than the best one found so far by the algorithm. [wiki](https://en.wikipedia.org/wiki/Branch_and_bound)


```
./teeline branch_bound --verbose
./teeling branch_bound -i ./data/discopt/tsp_5_1.tsp
```

###### Resources


* Backtracking from Skiena's "Algorithm Design Manual": http://www.algorist.com/algorist.html
* EECS 281: S20 Lecture 21 - Backtracking and Branch & Bound (Traveling Salesperson Problem) - https://www.youtube.com/watch?v=hNs7G1b2iFY&t=5480s

* GeekForGeeks article: https://www.geeksforgeeks.org/traveling-salesman-problem-using-branch-and-bound-2/
* Optimization Methods, Lecture.13, https://ocw.mit.edu/courses/sloan-school-of-management/15-093j-optimization-methods-fall-2009/lecture-notes/

#### Bellman-Karp-Held

The Held–Karp algorithm, also called **Bellman–Held–Karp** algorithm, is a dynamic programming algorithm proposed in 1962 independently by Bellman[1] and by Held and Karp[2] to solve the Traveling Salesman Problem (TSP). [wiki](https://en.wikipedia.org/wiki/Held%E2%80%93Karp_algorithm)

1. Compute the solutions of all subproblems starting with the smallest.
2. Whenever computing a solution requires solutions for smaller problems using the above recursive equations, look up these solutions which are already computed.
3. To compute a minimum distance tour, use the final equation to generate the 1st node, and repeat for the other nodes. For this problem, we cannot know which subproblems we need to solve, so we solve them all.


```
./teeline bellman_karp  -i ./data/discopt/tsp_5_1.tsp
./teeline bhk
./teeline bhk --verbose
```

###### Resources

* Bellman-Held-Karp: https://youtu.be/D8aHqaFa8GE
* Algorithms Illuminated, part.4 - https://www.amazon.com/dp/B08D4T91RL


### Approximate algorithms:

In computer science and operations research, approximation algorithms are efficient algorithms that find approximate solutions to optimization problems (in particular NP-hard problems) with provable guarantees on the distance of the returned solution to the optimal one. [wiki](https://en.wikipedia.org/wiki/Approximation_algorithm)


#### greedy nearest neighbors using KD-tree

It iterates over list of cities and selects the closest neighbor as next city.
This implementation uses KD-tree for a lookup.

```
./teeline nn
./teeline nn --verbose
```

###### Resources

* "Algorithms and Data Structures in Action", https://www.manning.com/books/algorithms-and-data-structures-in-action
* "Nearest neighbor algorithm", https://en.wikipedia.org/wiki/Nearest_neighbour_algorithm



#### 2-opt heuristic

In optimization, 2-opt is a simple local search algorithm for solving the traveling salesman problem. 
The main idea behind it is to take a route that crosses over itself and reorder it so that it does not. 

```
./teeline two_opt
./teeline 2opt

./target/debug/bin 2opt -i ./data/discopt/tsp_5_1.tsp
```

###### Resources

* Section 20.4: The 2-OPT Heuristic for the TSP - https://youtu.be/dYEWqrp-mho
* 2-opt wiki: https://en.wikipedia.org/wiki/2-opt
* The Traveling Salesman Problem:A Case Study in Local Optimization, https://www.cs.ubc.ca/~hutter/previous-earg/EmpAlgReadingGroup/TSP-JohMcg97.pdf



##### stochastic hill climbing

It is an iterative algorithm that starts with an arbitrary solution to a problem, then attempts to find a better solution by making an incremental change to the solution. If the change produces a better solution, another incremental change is made to the new solution, and so on until no further improvements can be found.

The solver implements a version called Hill Climbing with random restarts to avoid getting stuck on plateu or local maxima;


Possible metaheuristics for tuning:

* `verbose` - prints some debugging details onto std-out

* `epochs` - max iterations, if 0 then it would run forever

* `platoo_epochs` - how long to keep walking without any progress

```
./teeline stochastic_hill
./teeline stochastic_hill --epochs=100
./teeling stochastic_hill --platoo_epochs=10
```

###### Resources

* AIMA, Chapter 4.1 Local Search and Optimization Problems, http://aima.cs.berkeley.edu/contents.html
* wiki: https://en.wikipedia.org/wiki/Hill_climbing

##### simulated annealing

TODO

Simulated annealing (SA) is a probabilistic technique for approximating the global optimum of a given function.


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

###### Resources

* AIMA, 4.1.2   Simulated annealing, http://aima.cs.berkeley.edu/contents.html
* wiki, Simulated annealing, https://en.wikipedia.org/wiki/Simulated_annealing

##### tabu search

Tabu search enhances the performance of local search by relaxing its basic rule. First, at each step worsening moves can be accepted if no improving move is available (like when the search is stuck at a strict local minimum). In addition, prohibitions (henceforth the term tabu) are introduced to discourage the search from coming back to previously-visited solutions.

available options:

* `epochs` - how many iterations to run before giving up

```
./teeline tabu_search --epochs=5
```

###### Resources

* Wiki, https://en.wikipedia.org/wiki/Tabu_search
* Heuristic Search, chapter 14.4. Tabu Search, https://learning.oreilly.com/library/view/heuristic-search/9780123725127/B9780123725127000146.xhtml
* AIMA 3rd Edition

##### genetic search


In computer science and operations research, a genetic algorithm (GA) is a metaheuristic inspired by the process of natural selection that belongs to the larger class of evolutionary algorithms (EA).

available metaheuristics:

* `epochs` - limits the max number of iteration before stopping the search, default 10.000

* `mutation_probability` - sets the probability of applying random mutation for new child, default 0.001

* `n_elite` - how many individuals of each population should sent directly to next generation, default 3

```
./teeline genetic_algorithm
./teeline ga --verbose
./teeline ga --epochs = 5 --mutation_probability = 0.2
./teeline ga --n_elite = 7
```

###### Resources

* Ch.4.1.4 - "Genetic Algorithms", AIMA, https://github.com/aimacode/aima-python/blob/ca301ea363674ec719b58f23e794998de4f623c9/search.py#L912
* Ch7.5 - Random sampling for local search, Skiena: https://books.google.de/books?id=7XUSn0IKQEgC&lpg=PR1&pg=PA251#v=onepage&q&f=false
* ch.4 "TSP", Genetic Algorithms in Java: https://learning.oreilly.com/library/view/genetic-algorithms-in/9781484203286/9781484203293_Ch04.xhtml
* comparitions of selection https://arxiv.org/ftp/arxiv/papers/1203/1203.3099.pdf
* ordered crossover: http://www.dmi.unict.it/mpavone/nc-cs/materiale/moscato89.pdf
* comparition of crossover methods: http://www.iro.umontreal.ca/~dift6751/ga_tsp_tr.pdf
* Wiki, https://en.wikipedia.org/wiki/Genetic_algorithm


