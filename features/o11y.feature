Feature: OpenTelemetry metrics
  Scenario: Worker metrics published
    Given Worker is configured to point to mock Cloudflare API
    Given Worker is configured to send metrics to a mock OpenTelemetry collector
    When  Worker is triggered
    Then  Worker metrics are published
    And   Meter name should include "cloudflare_worker_requests"
