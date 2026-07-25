# POST /api/v1/pipeline — nn seeds a 2-opt cleanup stage
POST http://127.0.0.1:8080/api/v1/pipeline
Content-Type: application/json
```json
{
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 1.0, "y": 1.0},
      {"x": 0.0, "y": 1.0}
    ]
  },
  "stages": [
    {"solver": "nn"},
    {"solver": "2opt"}
  ]
}
```
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$.stages" isCollection
jsonpath "$.stages" count == 2
jsonpath "$.stages[0].solver" == "nn"
jsonpath "$.stages[1].solver" == "2opt"
jsonpath "$.final_cost" isFloat
jsonpath "$.final_tour" isCollection
jsonpath "$.warnings" isCollection


# POST /api/v1/pipeline — nn appearing after stage 0 surfaces a warning
POST http://127.0.0.1:8080/api/v1/pipeline
Content-Type: application/json
```json
{
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 1.0, "y": 1.0},
      {"x": 0.0, "y": 1.0}
    ]
  },
  "stages": [
    {"solver": "2opt"},
    {"solver": "nn"}
  ]
}
```
HTTP 200
[Asserts]
jsonpath "$.warnings" count >= 1


# POST /api/v1/pipeline — fewer than 2 stages returns 400
POST http://127.0.0.1:8080/api/v1/pipeline
Content-Type: application/json
```json
{
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0}
    ]
  },
  "stages": [
    {"solver": "nn"}
  ]
}
```
HTTP 400
[Asserts]
jsonpath "$.error" isString


# POST /api/v1/pipeline — unknown solver in a later stage still fails the
# whole request (fail-fast, no partial results)
POST http://127.0.0.1:8080/api/v1/pipeline
Content-Type: application/json
```json
{
  "input": {
    "cities": [
      {"x": 0.0, "y": 0.0},
      {"x": 1.0, "y": 0.0},
      {"x": 1.0, "y": 1.0}
    ]
  },
  "stages": [
    {"solver": "nn"},
    {"solver": "does_not_exist"}
  ]
}
```
HTTP 400
[Asserts]
jsonpath "$.error" isString


# POST /api/v1/pipeline — both input fields returns 400
POST http://127.0.0.1:8080/api/v1/pipeline
Content-Type: application/json
```json
{
  "input": {
    "cities": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 0.0}],
    "tsplib": "NAME: x\n"
  },
  "stages": [
    {"solver": "nn"},
    {"solver": "2opt"}
  ]
}
```
HTTP 400
