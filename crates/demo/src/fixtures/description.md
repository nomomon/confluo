Webhook deliveries currently fail on the first non-2xx response, so a short outage on the receiving side drops events for good. This retries transient failures with capped exponential backoff and full jitter.

## What changes

| Status | Before | After |
| --- | --- | --- |
| `2xx` | delivered | delivered |
| `408`, `429`, `5xx` | dropped | retried up to 8 times |
| other `4xx` | dropped | dropped, logged |

Endpoints can override the policy:

```rust
let endpoint = Endpoint {
    url: "https://hooks.example.com/acme".into(),
    secret,
    retry: Some(RetryPolicy { max_attempts: 5, ..Default::default() }),
};
```

## Testing

- [x] Unit tests for the delay cap and retryable statuses
- [x] Replayed a day of staging traffic against a flaky receiver
- [ ] Dashboard for retry counts (follow-up)
