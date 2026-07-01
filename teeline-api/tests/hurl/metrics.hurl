# GET /metrics — Prometheus/OpenMetrics scrape endpoint
GET http://127.0.0.1:8080/metrics
HTTP 200
[Asserts]
header "Content-Type" contains "openmetrics-text"
body contains "# HELP http_requests"
body contains "# HELP teeline_solver_requests"
body contains "# HELP teeline_solver_duration_seconds"
