# Automation with Triggers

AgentOS supports cron-based triggers that run tasks on a schedule without manual intervention.

## Creating a Trigger

### From the UI
1. Navigate to **Automation > Triggers**.
2. Click **New Trigger**.
3. Enter a name, cron expression, and the task text.
4. Click **Save**.

### From the API
```bash
curl -X POST http://localhost:8080/v1/triggers \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{
    "name": "daily-health",
    "cron": "0 9 * * *",
    "task": "run system health check and email report"
  }'
```

## Cron Expression Reference

| Expression      | Schedule                    |
|-----------------|-----------------------------|
| `* * * * *`     | Every minute                |
| `0 * * * *`     | Every hour                  |
| `0 9 * * *`     | Daily at 9:00 AM            |
| `0 9 * * 1-5`   | Weekdays at 9:00 AM         |
| `0 0 1 * *`     | First day of each month     |
| `*/5 * * * *`   | Every 5 minutes             |

Format: `minute hour day-of-month month day-of-week`

## Managing Triggers

### List Triggers
```bash
curl http://localhost:8080/v1/triggers \
  -H "Authorization: Bearer aos_yourkey"
```

### Disable a Trigger
```bash
curl -X PATCH http://localhost:8080/v1/triggers/daily-health \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{"enabled": false}'
```

### Delete a Trigger
```bash
curl -X DELETE http://localhost:8080/v1/triggers/daily-health \
  -H "Authorization: Bearer aos_yourkey"
```

## Trigger Conditions

You can add conditions so a trigger only fires when criteria are met:

```json
{
  "name": "low-disk-alert",
  "cron": "*/30 * * * *",
  "task": "check disk space and alert if below 10%",
  "condition": {
    "type": "threshold",
    "metric": "disk_free_percent",
    "operator": "lt",
    "value": 10
  }
}
```

## Chaining Triggers with Playbooks

Triggers can reference playbooks instead of single tasks:

```json
{
  "name": "morning-routine",
  "cron": "0 8 * * 1-5",
  "playbook": "daily-health"
}
```

## Logs and History

Trigger execution history is available at **Automation > Trigger History** or via API:

```bash
curl http://localhost:8080/v1/triggers/daily-health/history \
  -H "Authorization: Bearer aos_yourkey"
```
