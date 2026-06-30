GET http://127.0.0.1:8080/api/v1/solvers
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$" isCollection
jsonpath "$" count > 0
jsonpath "$[*].name" isCollection
jsonpath "$[*].alias" isCollection
jsonpath "$[*].category" isCollection
jsonpath "$[*].desc" isCollection
jsonpath "$[*].complexity" isCollection
jsonpath "$[*].alias" includes "nn"
