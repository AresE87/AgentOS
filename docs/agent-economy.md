# Agent Economy (v4.2.0)

AgentOS v4.2.0 introduces a full agent economy: creators build and publish agents, users hire them, payments flow through escrow, and reputation drives discovery.

> "AgentOS isn't just software. It's an economy."

## Modules

### Agent Hiring (R141)
Post jobs with requirements and budget. Agents apply, creators review applicants, and hire the best fit. Pricing models: per-task, per-hour, monthly subscription, or pay-as-you-go.

**IPC Commands:** `hiring_post`, `hiring_list`, `hiring_apply`, `hiring_hire`

### Reputation System (R142)
Every agent has a reputation score derived from user reviews, task success rate, and response time. Badges are awarded for milestones (1000+ tasks, 98%+ success rate). A leaderboard ranks top agents.

**IPC Commands:** `reputation_get`, `reputation_review`, `reputation_leaderboard`

### Cross-User Collaboration (R143)
Users create shared project rooms where agents from different users collaborate. Participants contribute agents, share results, and track progress in real-time.

**IPC Commands:** `collab_create`, `collab_join`, `collab_list`, `collab_share`

### Microtasks Marketplace (R144)
Agents offer micro-services at low prices: translation per word, code review per file, data extraction per page. Users post tasks, agents claim and complete them.

**IPC Commands:** `microtask_post`, `microtask_claim`, `microtask_complete`, `microtask_list`

### Escrow (R145)
For high-value tasks, payment is held in escrow until the user approves the result. Supports release, refund, and dispute workflows. Auto-accept after 72 hours.

**IPC Commands:** `escrow_create`, `escrow_release`, `escrow_refund`, `escrow_list`

### Agent Insurance (R146)
Coverage tiers from Basic (free, $100 limit) to Enterprise ($100K limit). Users file claims with evidence when an agent causes verified damage. Claims go through review and approval.

**IPC Commands:** `insurance_create`, `insurance_list`, `insurance_claim`, `insurance_status`

### Creator Studio (R147)
Full IDE for building premium agents. Create projects (playbooks, personas, plugins, templates), edit, test with live preview, and publish to the marketplace with version management.

**IPC Commands:** `creator_create`, `creator_publish`, `creator_list`, `creator_analytics`

### Creator Analytics (R148)
Business intelligence for creators: total downloads, revenue history, download trends, top products, and net revenue after commission (30%).

**IPC Commands:** `creator_metrics`, `creator_revenue`, `creator_trends`

### Affiliate Program (R149)
Referral link tracking with tiered commissions. Starter (10%), Partner (15%), Champion (20%), Ambassador (25%). Automatic tier upgrades based on conversion count.

**IPC Commands:** `affiliate_create`, `affiliate_earnings`, `affiliate_list`, `affiliate_track`

## Architecture

All economy modules live under `src-tauri/src/economy/` with in-memory state managers. Each module exposes Tauri IPC commands registered in the invoke handler, and frontend hooks in `useAgent.ts`.

## Frontend Integration

All hooks are available via `useAgent()`:

```typescript
const {
  hiringPost, hiringList, hiringApply, hiringHire,
  reputationGet, reputationReview, reputationLeaderboard,
  collabCreate, collabJoin, collabList, collabShare,
  microtaskPost, microtaskClaim, microtaskComplete, microtaskList,
  escrowCreate, escrowRelease, escrowRefund, escrowList,
  insuranceCreate, insuranceList, insuranceClaim, insuranceStatus,
  creatorCreate, creatorPublish, creatorList, creatorAnalytics,
  creatorMetrics, creatorRevenue, creatorTrends,
  affiliateCreate, affiliateEarnings, affiliateList, affiliateTrack,
} = useAgent();
```
