# POST /api/v1/compare — two solvers
POST http://127.0.0.1:8080/api/v1/compare
Content-Type: application/json
```json
{
  "solvers": ["nn", "2opt"],
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
jsonpath "$.entries" isCollection
jsonpath "$.entries" count == 2
jsonpath "$.entries[*].status" isCollection
jsonpath "$.entries[*].solver" isCollection


# POST /api/v1/compare — fewer than 2 solvers returns 400
POST http://127.0.0.1:8080/api/v1/compare
Content-Type: application/json
```json
{
  "solvers": ["nn"],
  "input": {
    "cities": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 0.0}, {"x": 0.5, "y": 1.0}]
  }
}
```
HTTP 400


# POST /api/v1/compare — empty solvers returns 400
POST http://127.0.0.1:8080/api/v1/compare
Content-Type: application/json
```json
{"solvers": [], "input": {"cities": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 0.0}]}}
```
HTTP 400
