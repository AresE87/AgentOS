# Smart Playbooks

Smart Playbooks extend basic playbooks with variables, conditionals, loops, and error handling.

## Basic Playbook Recap

```yaml
name: Simple Health Check
steps:
  - task: check disk space
  - task: check CPU usage
```

## Variables

Define variables at the top and reference them with `{{ var }}` syntax:

```yaml
name: Server Check
variables:
  server: production-01
  threshold: 80
steps:
  - task: check CPU usage on {{ server }}
  - task: alert if CPU above {{ threshold }}%
```

### Runtime Variables

Pass variables when running a playbook:

```bash
curl -X POST http://localhost:8080/v1/playbook/run \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{
    "playbook": "server-check",
    "variables": {
      "server": "staging-02",
      "threshold": 90
    }
  }'
```

## Conditionals

Use `when` to conditionally execute steps:

```yaml
name: Conditional Cleanup
variables:
  environment: production
steps:
  - task: check disk space
    id: disk_check

  - task: delete temp files older than 7 days
    when: "{{ disk_check.result.free_percent }} < 20"

  - task: send cleanup report to ops team
    when: "{{ environment }} == 'production'"
```

## Loops

Iterate over a list with `for_each`:

```yaml
name: Multi-Server Check
variables:
  servers:
    - web-01
    - web-02
    - db-01
steps:
  - task: check health of {{ item }}
    for_each: "{{ servers }}"

  - task: summarize health check results
```

## Error Handling

### Continue on Failure
```yaml
steps:
  - task: restart service-a
    on_failure: continue

  - task: restart service-b
    on_failure: continue

  - task: verify all services running
```

### Retry Logic
```yaml
steps:
  - task: deploy latest build
    retry:
      max_attempts: 3
      delay_seconds: 10
    on_failure: abort
```

### Fallback Tasks
```yaml
steps:
  - task: deploy via blue-green strategy
    id: deploy
    on_failure: continue

  - task: rollback to previous version
    when: "{{ deploy.status }} == 'failed'"
```

## Step Outputs and Chaining

Each step stores its output. Reference previous steps using `$prev` or the step `id`:

```yaml
name: Analyze and Report
steps:
  - task: scan open ports on 192.168.1.0/24
    id: port_scan

  - task: identify risky open ports from this list — {{ port_scan.result }}
    id: analysis

  - task: generate security report — {{ analysis.result }}
```

## Parallel Steps

Run steps concurrently with `parallel`:

```yaml
name: Parallel Health Check
steps:
  - parallel:
      - task: check disk space
      - task: check CPU usage
      - task: check memory usage

  - task: summarize all results from above
```

## Complete Example

```yaml
name: Weekly Maintenance
variables:
  notify_email: ops@example.com
  disk_threshold: 20

steps:
  - parallel:
      - task: check disk space
        id: disk
      - task: check for OS updates
        id: updates

  - task: clean temp files
    when: "{{ disk.result.free_percent }} < {{ disk_threshold }}"
    on_failure: continue

  - task: install critical security updates
    when: "{{ updates.result.critical_count }} > 0"
    retry:
      max_attempts: 2
      delay_seconds: 30

  - task: send maintenance report to {{ notify_email }}
```
