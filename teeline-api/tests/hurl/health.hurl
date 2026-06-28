GET http://127.0.0.1:8080/api/v1/health
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$.status" == "ok"
jsonpath "$.version" == "0.1.0"
