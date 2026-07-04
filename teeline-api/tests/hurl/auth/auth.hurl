# GET /api/v1/health — always exempt, no header needed even when API_KEY is set
GET http://127.0.0.1:8080/api/v1/health
HTTP 200
[Asserts]
jsonpath "$.status" == "ok"


# GET /api/v1/solvers — no header returns 401
GET http://127.0.0.1:8080/api/v1/solvers
HTTP 401
[Asserts]
jsonpath "$.error" == "Unauthorized"


# GET /api/v1/solvers — wrong Authorization: Bearer token returns 401
GET http://127.0.0.1:8080/api/v1/solvers
Authorization: Bearer wrong-token
HTTP 401


# GET /api/v1/solvers — wrong X-Api-Key returns 401
GET http://127.0.0.1:8080/api/v1/solvers
X-Api-Key: wrong-token
HTTP 401


# POST /api/v1/parse — valid body but no header returns 401
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
HTTP 401


# POST /api/v1/solve — valid body but no header returns 401
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
HTTP 401


# GET /api/v1/solvers — correct Authorization: Bearer token returns 200
GET http://127.0.0.1:8080/api/v1/solvers
Authorization: Bearer {{api_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# GET /api/v1/solvers — correct X-Api-Key returns 200
GET http://127.0.0.1:8080/api/v1/solvers
X-Api-Key: {{api_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# POST /api/v1/parse — correct Authorization: Bearer token returns 200
POST http://127.0.0.1:8080/api/v1/parse
Authorization: Bearer {{api_token}}
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
jsonpath "$.cities" count == 3


# POST /api/v1/solve — correct X-Api-Key returns 200
POST http://127.0.0.1:8080/api/v1/solve
X-Api-Key: {{api_token}}
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
jsonpath "$.solver" == "nn"
