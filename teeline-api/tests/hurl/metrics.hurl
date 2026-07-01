# GET /metrics — Prometheus/OpenMetrics scrape endpoint
# Prime solver metrics with one solve before checking — hurl runs files in parallel.
POST http://127.0.0.1:8080/api/v1/solve
Content-Type: application/json
{"input":{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":0.5,"y":1.0}]},"solver":"nn"}
HTTP 200

GET http://127.0.0.1:8080/metrics
HTTP 200
[Asserts]
header "Content-Type" contains "openmetrics-text"
body contains "http_requests_total"
body contains "teeline_solver_requests_total"
body contains "teeline_solver_duration_seconds"
