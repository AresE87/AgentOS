# AgentOS v4.0 -- Intelligent Agent

## Overview

AgentOS v4.0 introduces a comprehensive reasoning and intelligence layer that enables the agent to think more deeply, learn from experience, and improve over time. This release adds 10 new capabilities spanning reasoning chains, self-correction, multimodal analysis, causal inference, knowledge graphs, hypothesis generation, confidence calibration, transfer learning, and meta-learning.

## Architecture

All intelligence modules live under `src-tauri/src/reasoning/` and `src-tauri/src/knowledge/`:

```
reasoning/
  chains.rs          -- R121: Step-by-step reasoning chains
  self_correction.rs -- R122: Automatic output verification and correction
  multimodal.rs      -- R123: Cross-modal evidence analysis
  causal.rs          -- R124: Causal graphs and counterfactual reasoning
  hypothesis.rs      -- R126: Hypothesis generation and evaluation
  confidence.rs      -- R127: Confidence calibration and tracking
  transfer.rs        -- R128: Cross-domain pattern transfer
  meta_learning.rs   -- R129: Domain learning curves and accuracy prediction

knowledge/
  graph.rs           -- R125: SQLite-backed entity-relationship knowledge graph
```

## Reasoning Capabilities (R121-R124)

### Reasoning Chains (R121)
Break complex tasks into sequential reasoning steps. Each step records a thought, conclusion, and confidence score. Chains are stored in memory and can be reviewed for transparency.

### Self-Correction (R122)
After producing an output, the agent can verify its own work and apply corrections. Tracks correction rounds to measure improvement.

### Multimodal Reasoning (R123)
Combine evidence from multiple modalities (text, images, structured data) into a unified analysis with weighted confidence.

### Causal Inference (R124)
Build causal graphs from observed relationships. Run counterfactual analyses ("what if X had not happened?") to understand root causes.

## Knowledge Management (R125)

### Knowledge Graph
Persistent SQLite-backed graph of entities and relationships. Supports typed entities, relationship traversal, and full-text search. Enables the agent to accumulate and query structured knowledge over time.

## Intelligence Layer (R126-R129)

### Hypothesis Generation (R126)
Given a question, the engine generates multiple competing hypotheses with initial probability estimates. As evidence is gathered, probabilities update using a Bayesian-like rule (supporting evidence multiplies by 1.3, contradicting evidence by 0.7, clamped to [0,1]). Hypotheses automatically transition to "confirmed" (>0.9) or "rejected" (<0.1).

**IPC Commands:**
- `cmd_hypothesis_generate` -- Generate hypotheses for a question
- `cmd_hypothesis_update` -- Update probability with new evidence
- `cmd_hypothesis_get` -- Get a specific hypothesis
- `cmd_hypothesis_list` -- List hypotheses by probability

### Confidence Calibration (R127)
Track confidence scores for every task and compare predicted confidence against actual outcomes. Produces calibration statistics showing whether the agent is overconfident or underconfident. Tasks with confidence below 0.6 are flagged for auto-verification.

**IPC Commands:**
- `cmd_confidence_record` -- Record a confidence score (and optionally outcome)
- `cmd_confidence_calibration` -- Get calibration statistics
- `cmd_confidence_stats` -- Get average confidence and calibration overview

### Transfer Learning (R128)
Register patterns learned in one domain and apply them to new domains. Track how many times each pattern has been applied and whether it was helpful. Confidence adjusts based on outcomes.

**IPC Commands:**
- `cmd_transfer_register` -- Register a new learned pattern
- `cmd_transfer_find` -- Find patterns applicable to a domain
- `cmd_transfer_apply` -- Apply a pattern to a new domain
- `cmd_transfer_list` -- List all registered patterns

### Meta-Learning (R129)
Track learning curves across domains: how many tasks completed, corrections needed, accuracy percentage, and learning rate. Predict future accuracy based on historical improvement. Identify which domains the agent learns fastest in.

**IPC Commands:**
- `cmd_meta_record` -- Record a task outcome for a domain
- `cmd_meta_curve` -- Get the learning curve for a domain
- `cmd_meta_all_curves` -- Get all domain learning curves
- `cmd_meta_predict` -- Predict accuracy after N additional tasks

## Frontend Integration

All intelligence features are exposed through the `useAgent()` hook in `frontend/src/hooks/useAgent.ts`:

```typescript
const {
  // R126: Hypothesis Generation
  hypothesisGenerate, hypothesisUpdate, hypothesisGet, hypothesisList,
  // R127: Confidence Calibration
  confidenceRecord, confidenceCalibration, confidenceStats,
  // R128: Transfer Learning
  transferRegister, transferFind, transferApply, transferList,
  // R129: Meta-Learning
  metaRecord, metaCurve, metaAllCurves, metaPredict,
} = useAgent();
```

## Storage

- **In-memory:** Reasoning chains, self-correction, multimodal analysis, causal graphs, hypotheses, transfer patterns
- **SQLite:** Knowledge graph, confidence scores, domain learning curves
