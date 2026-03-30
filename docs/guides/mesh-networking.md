# Mesh Networking

AgentOS Mesh connects multiple PCs into a peer-to-peer network, enabling cross-machine task execution and resource sharing.

## Overview

- Each AgentOS instance is a **node** in the mesh.
- Nodes discover each other via mDNS on the local network or manual peer configuration.
- Tasks can be routed to the node best suited to execute them.

## Enabling Mesh

### From the UI
1. Go to **Settings > Mesh Network**.
2. Toggle **Enable Mesh** on.
3. Set a **Mesh Name** (all nodes sharing a name form one mesh).
4. Click **Save** and restart AgentOS.

### From the Config File
Edit `config/settings.json`:
```json
{
  "mesh": {
    "enabled": true,
    "mesh_name": "my-home-lab",
    "listen_port": 9100,
    "discovery": "mdns"
  }
}
```

## Adding Peers Manually

If mDNS is not available (different subnets, cloud VMs), add peers by IP:

```json
{
  "mesh": {
    "enabled": true,
    "discovery": "static",
    "peers": [
      "192.168.1.50:9100",
      "10.0.0.12:9100"
    ]
  }
}
```

## Sending Tasks to Specific Nodes

```bash
curl -X POST http://localhost:8080/v1/message \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{
    "text": "check GPU temperature",
    "target_node": "workstation-02"
  }'
```

If `target_node` is omitted, the mesh routes the task automatically based on node capabilities and load.

## Node Capabilities

Each node advertises its capabilities:

```json
{
  "node_name": "workstation-02",
  "capabilities": ["gpu", "docker", "large-disk"],
  "os": "windows",
  "cpu_cores": 16,
  "ram_gb": 64
}
```

Tasks requiring specific capabilities (e.g., GPU) are routed to nodes that have them.

## Security

- All mesh traffic is encrypted with TLS 1.3.
- Nodes authenticate using a shared mesh secret.
- Set the secret in config:
  ```json
  { "mesh": { "secret": "your-shared-secret-here" } }
  ```

## Monitoring the Mesh

### List Connected Nodes
```bash
curl http://localhost:8080/v1/mesh/nodes \
  -H "Authorization: Bearer aos_yourkey"
```

### Check Mesh Health
```bash
curl http://localhost:8080/v1/mesh/health \
  -H "Authorization: Bearer aos_yourkey"
```

## Relay Mode

For nodes behind NAT or firewalls, AgentOS supports relay mode. A publicly accessible node acts as a relay:

```json
{
  "mesh": {
    "relay": {
      "enabled": true,
      "relay_url": "wss://relay.example.com:9100"
    }
  }
}
```
