GET http://127.0.0.1:8080/api/v1/solvers
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$" isCollection
jsonpath "$" count > 0
jsonpath "$[0].name" isString
jsonpath "$[0].alias" isString
jsonpath "$[0].category" isString
jsonpath "$[0].desc" isString
jsonpath "$[0].complexity" isString
jsonpath "$[0].has_options" isBoolean
jsonpath "$[0].exact" isBoolean
jsonpath "$[*].alias" includes "nn"
