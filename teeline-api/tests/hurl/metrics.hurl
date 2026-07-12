# POST /api/v1/solve — populate solver metrics before scraping /metrics.
# Solver metric families are empty (and therefore omitted from the OpenMetrics
# output by prometheus-client) until at least one solve request has occurred.
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
HTTP 200


# GET /metrics — Prometheus/OpenMetrics scrape endpoint
GET http://127.0.0.1:8080/metrics
HTTP 200
[Asserts]
header "Content-Type" contains "openmetrics-text"
body contains "# HELP http_requests"
body contains "# HELP teeline_solver_requests"
body contains "# HELP teeline_solver_duration_seconds"
