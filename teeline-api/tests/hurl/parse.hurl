# POST /api/v1/parse — TSPLIB input
POST http://127.0.0.1:8080/api/v1/parse
Content-Type: application/json
```json
{
  "input": {
    "tsplib": "NAME: tiny\nTYPE: TSP\nCOMMENT: three cities\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0.0 0.0\n2 1.0 0.0\n3 0.5 1.0\nEOF\n"
  }
}
```
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$.name" == "tiny"
jsonpath "$.distance_type" == "EUC_2D"
jsonpath "$.cities" isCollection
jsonpath "$.cities" count == 3
jsonpath "$.cities[*].id" isCollection
jsonpath "$.cities[*].x" isCollection
jsonpath "$.cities[*].y" isCollection


# POST /api/v1/parse — JSON cities input
POST http://127.0.0.1:8080/api/v1/parse
Content-Type: application/json
```json
{
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
jsonpath "$.distance_type" == "EUC_2D"
jsonpath "$.cities" count == 3


# POST /api/v1/parse — both fields returns 400
POST http://127.0.0.1:8080/api/v1/parse
Content-Type: application/json
```json
{
  "input": {
    "tsplib": "NAME: x\n",
    "cities": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 0.0}]
  }
}
```
HTTP 400


# POST /api/v1/parse — neither field returns 400
POST http://127.0.0.1:8080/api/v1/parse
Content-Type: application/json
```json
{"input": {}}
```
HTTP 400
