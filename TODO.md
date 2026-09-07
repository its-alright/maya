Реализовать полноценные gRPC клиенты вместо заглушек
Добавить Circuit Breaker через tower::retry
Настроить автоматические миграции БД
Добавить кэширование (Redis)
Написать интеграционные тесты
Добавить CI/CD (GitHub Actions)

# Metrics
```sql
rps + errors
sum by (method, path) (rate(http_requests_total[$__rate_interval]))
sum by (method, path) (rate(http_errors_total[$__rate_interval]))

percentile
histogram_quantile(1.0, _bucket{})
histogram_quantile(0.99, sum(rate(_bucket[$__rate_interval])) by (le, method, path)) * 1000
p.99={method}{path}
```