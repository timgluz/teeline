# GET /openapi.json — serves OpenAPI 3.1.0 spec as JSON
GET http://127.0.0.1:8080/openapi.json
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$.openapi" == "3.1.0"
jsonpath "$.paths" isCollection
jsonpath "$.components.schemas" isCollection

# GET /docs — serves Scalar interactive docs UI
GET http://127.0.0.1:8080/docs
HTTP 200
[Asserts]
header "Content-Type" contains "text/html"
body contains "<!doctype html>"
