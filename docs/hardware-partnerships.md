# AgentOS Hardware Partnership Program

## Overview

The AgentOS Hardware Partnership Program enables PC and device manufacturers to
pre-install AgentOS on their hardware, providing an out-of-the-box AI assistant
experience for end users.

---

## Partnership Tiers

### Basic
- Silent OEM installer included on device
- Default AgentOS branding
- Free tier pre-activated for end users
- Basic telemetry dashboard access
- Standard support SLA (48h)

### Premium
- Custom-branded installer ("Dell AI Assistant powered by AgentOS")
- Configurable logo, accent color, welcome message
- Priority activation for end users (extended trial)
- Full OEM analytics dashboard (installs, activations, upgrades)
- Premium support SLA (24h)
- Revenue share on Pro upgrades

### Exclusive
- Deep OS-level integration (startup, taskbar pinning, default assistant)
- Co-developed features specific to hardware line
- Joint marketing and press releases
- Dedicated partner engineering team
- Exclusive support SLA (4h)
- Enhanced revenue share + per-unit license fee

---

## Requirements

| Requirement           | Basic   | Premium  | Exclusive |
|-----------------------|---------|----------|-----------|
| Minimum units/year    | 5,000   | 50,000   | 500,000   |
| Technical integration | Installer | Branded  | OS-level  |
| Certification testing | Self    | Assisted | Joint     |
| Marketing commitment  | None    | Co-brand | Joint PR  |

---

## Revenue Model

- **Option A** -- License fee: $1-3 per unit (volume discounts available)
- **Option B** -- Revenue share: percentage of Free-to-Pro upgrades from OEM installs
- **Option C** -- Hybrid: $0.50 per unit + 20% revenue share on upgrades

---

## Certification Process

1. **Application** -- Submit partnership application with device specs
2. **Integration** -- Install AgentOS OEM package on target hardware
3. **Testing** -- Run the AgentOS Hardware Compatibility Test Suite
4. **Review** -- AgentOS team reviews test results and integration quality
5. **Certification** -- Receive "AgentOS Certified" badge for marketing materials

---

## OEM Installer

The OEM installer supports the following flags:

```
agentos-oem-installer.exe
  --silent                     # No user interaction required
  --brand-name "Partner AI"    # Custom product name
  --brand-logo assets/logo.png # Custom logo
  --brand-color "#0076CE"      # Accent color (hex)
  --pre-configure free         # Pre-activate free tier
  --first-run-wizard true      # Show setup wizard on first launch
```

---

## Contact

To apply for the Hardware Partnership Program, email partnerships@agentos.dev
or visit https://agentos.dev/partners.
