# AgentOS Zapier Webhook Integration

Connect AgentOS to thousands of apps using Zapier webhooks.

## Overview

Zapier does not have a native AgentOS integration (yet), but you can use **Webhooks by Zapier** to call the AgentOS API from any Zap.

## Setup: Zapier to AgentOS (Trigger an AgentOS Task)

### Step 1: Create a New Zap
1. Log in to [zapier.com](https://zapier.com) and click **Create Zap**.

### Step 2: Choose Your Trigger
Pick any trigger app (e.g., Gmail, Slack, GitHub, Google Sheets).

Example: **Gmail > New Email Matching Search** with search query `label:urgent`.

### Step 3: Add a Webhooks Action
1. Click **+** to add an action step.
2. Search for **Webhooks by Zapier**.
3. Choose **Custom Request**.

### Step 4: Configure the Webhook

| Field             | Value                                                  |
|-------------------|--------------------------------------------------------|
| **Method**        | `POST`                                                 |
| **URL**           | `http://YOUR_AGENTOS_HOST:8080/v1/message`             |
| **Data Pass-Through** | No                                                |
| **Data**          | `{"text": "New urgent email from {{from}} — {{subject}}. Summarize and draft reply."}` |
| **Headers**       | `Content-Type: application/json`                       |
|                   | `Authorization: Bearer aos_yourkey`                    |

### Step 5: Test and Enable
1. Click **Test step** to verify the webhook reaches AgentOS.
2. Verify the task appears in AgentOS.
3. Turn on the Zap.

## Setup: AgentOS to Zapier (Send Data from AgentOS)

You can configure AgentOS to call a Zapier Catch Hook when tasks complete.

### Step 1: Create a Zapier Catch Hook
1. Create a new Zap with trigger **Webhooks by Zapier > Catch Hook**.
2. Copy the webhook URL (e.g., `https://hooks.zapier.com/hooks/catch/123456/abcdef/`).

### Step 2: Configure AgentOS Webhook Output
Add a webhook output in your playbook:

```yaml
name: Report and Notify
steps:
  - task: generate daily sales report
    id: report

  - webhook:
      url: https://hooks.zapier.com/hooks/catch/123456/abcdef/
      method: POST
      body:
        report: "{{ report.result }}"
        timestamp: "{{ now }}"
```

### Step 3: Add Zapier Actions
After the Catch Hook trigger, add any Zapier actions:
- Send a Slack message with the report
- Create a Google Sheets row
- Send an email summary
- Post to a CRM

## Example Zap Ideas

| Trigger                        | AgentOS Task                                        |
|--------------------------------|-----------------------------------------------------|
| New GitHub issue               | Analyze the issue and suggest a fix                 |
| New row in Google Sheets       | Process the data entry and validate                 |
| Slack message in #ops          | Execute the requested operation                     |
| Calendar event starting        | Prepare meeting briefing and notes                  |
| New email with attachment      | Download, summarize, and file the attachment         |

## Security Notes

- Always use HTTPS for your AgentOS host when exposing to the internet.
- Store the API key in Zapier's built-in secret storage, not inline.
- Consider using a dedicated API key with limited permissions for Zapier integrations.
