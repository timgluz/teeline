# GET /metrics — Prometheus/OpenMetrics scrape endpoint
# NOTE: Run a POST /api/v1/solve request before this file so solver metrics
# are populated in the same server process. The Taskfile hurl runner should
# order solve.hurl before metrics.hurl.
GET http://127.0.0.1:8080/metrics
HTTP 200
[Asserts]
header "Content-Type" contains "openmetrics-text"
body contains "http_requests_total"
body contains "teeline_solver_requests_total"
body contains "teeline_solver_duration_seconds"
