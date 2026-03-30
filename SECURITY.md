# Security Policy

## Reporting Vulnerabilities

Email: security@agentos.app

Do NOT open public issues for security vulnerabilities.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x | Yes |
| < 1.0 | No |

## Security Features

- AES-256-GCM credential encryption (PBKDF2, 600K iterations)
- Command execution sandboxing with blocked patterns
- Input sanitization (XSS, SQL injection, path traversal)
- API rate limiting (per-plan tiers)
- Immutable audit log
- GDPR data export and erasure
