# GET / — index endpoint
GET http://127.0.0.1:8080/
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$.status" == "ok"
jsonpath "$.name" == "teeline-api"
jsonpath "$.routes" isCollection
jsonpath "$.routes" count > 0
