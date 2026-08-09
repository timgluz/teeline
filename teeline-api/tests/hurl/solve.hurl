# POST /api/v1/solve — nn solver with JSON cities
POST http://127.0.0.1:8080/api/v1/solve
Content-Type: application/json
```json
{
  "solver": "nn",
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 0.5, "y": 1.0}
    ]
  }
}
```
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$.solver" == "nn"
jsonpath "$.total" isFloat
jsonpath "$.route" isCollection
jsonpath "$.route" count == 3
jsonpath "$.duration_ms" isInteger


# POST /api/v1/solve — unknown solver returns 400
POST http://127.0.0.1:8080/api/v1/solve
Content-Type: application/json
```json
{
  "solver": "does_not_exist",
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 0.5, "y": 1.0}
    ]
  }
}
```
HTTP 400
[Asserts]
jsonpath "$.error" isString


# POST /api/v1/solve — both input fields returns 400
POST http://127.0.0.1:8080/api/v1/solve
Content-Type: application/json
```json
{
  "solver": "nn",
  "input": {
    "cities": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 0.0}],
    "tsplib": "NAME: x\n"
  }
}
```
HTTP 400


# POST /api/v1/solve — ACO with num_ants=0 returns 400
POST http://127.0.0.1:8080/api/v1/solve
Content-Type: application/json
```json
{
  "solver": "aco",
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 0.5, "y": 1.0}
    ]
  },
  "configs": {
    "aco": {"num_ants": 0}
  }
}
```
HTTP 400
[Asserts]
jsonpath "$.error" contains "num_ants"


# POST /api/v1/solve — SA with n_nearest=0 returns 400
POST http://127.0.0.1:8080/api/v1/solve
Content-Type: application/json
```json
{
  "solver": "sa",
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 0.5, "y": 1.0}
    ]
  },
  "configs": {
    "sa": {"heuristic": {"n_nearest": 0}}
  }
}
```
HTTP 400
[Asserts]
jsonpath "$.error" contains "n_nearest"


# POST /api/v1/solve — GA with negative mutation_probability returns 400
POST http://127.0.0.1:8080/api/v1/solve
Content-Type: application/json
```json
{
  "solver": "ga",
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 0.5, "y": 1.0}
    ]
  },
  "configs": {
    "ga": {"mutation_probability": -0.1}
  }
}
```
HTTP 400
[Asserts]
jsonpath "$.error" contains "mutation"
