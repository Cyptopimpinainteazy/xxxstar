"""Tests for x3_operator.telemetry"""

from x3_operator.telemetry import (
    Metric,
    MetricsRegistry,
    MetricType,
    create_operator_metrics,
    setup_structured_logging,
)


def test_counter():
    reg = MetricsRegistry()
    c = reg.counter("test_counter", "A test counter")
    c.increment()
    c.increment(5)
    assert c.value == 6.0


def test_gauge():
    reg = MetricsRegistry()
    g = reg.gauge("test_gauge", "A test gauge")
    g.set_value(42.5)
    assert g.value == 42.5


def test_prometheus_export():
    reg = MetricsRegistry()
    c = reg.counter("http_requests_total", "Total HTTP requests")
    c.increment(100)
    output = reg.export_prometheus()
    assert "http_requests_total" in output
    assert "100" in output
    assert "# TYPE http_requests_total counter" in output


def test_prometheus_labels():
    reg = MetricsRegistry()
    c = reg.counter("http_requests_total", labels={"method": "GET", "status": "200"})
    c.increment(10)
    output = reg.export_prometheus()
    assert 'method="GET"' in output
    assert 'status="200"' in output


def test_json_export():
    reg = MetricsRegistry()
    reg.counter("requests").increment(5)
    reg.gauge("uptime").set_value(120.0)
    output = reg.export_json()
    import json
    data = json.loads(output)
    assert "requests" in data
    assert "uptime" in data


def test_create_operator_metrics():
    reg = create_operator_metrics()
    # Should have at least 10 pre-registered metrics
    assert len(reg._metrics) >= 10


def test_setup_logging():
    log = setup_structured_logging(level="DEBUG", json_format=False)
    assert log.name == "x3_operator"
    assert log.level == 10  # DEBUG


def test_metric_to_prometheus():
    m = Metric(
        name="test_metric",
        metric_type=MetricType.GAUGE,
        value=42.0,
        help_text="A test",
    )
    output = m.to_prometheus()
    assert "# HELP test_metric A test" in output
    assert "# TYPE test_metric gauge" in output
    assert "test_metric 42.0" in output
