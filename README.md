# Compliance-Aware Block Builder

> **Pre-Execution Mempool Screening, 1-Hop Audit Graph Walks, & Asynchronous LLM Regulatory Narration**

A high-performance compliance layer for Ethereum block builders. Every transaction submitted to the mempool is evaluated before execution through a deterministic Rust policy engine backed by an in-memory Redis sanctions cache and a PostgreSQL transaction audit graph. Clean transactions proceed to block inclusion, directly sanctioned entities are blocked, and indirect counterparties are flagged for Enhanced Due Diligence (EDD). 

An asynchronous Gemini LLM agent narrates compliance decisions for audit reports after the fact—**the model explains decisions; it never makes them.**

---

## Architecture Overview

```mermaid
flowchart TD
    subgraph Client & Simulation
        TX[Incoming Transaction] -->|HTTP POST /screen| RE[Rust Compliance Engine]
    end

    subgraph Deterministic Screening Core
        RE -->|O(1) Direct Check| RD[(Redis Sanctions Cache)]
        RD -->|Hit: SANCTIONED_SENDER / RECIPIENT| B[Decision: BLOCK\nRisk: 98]
        RD -->|Miss| PG_CHK{Direct Hit?}
        PG_CHK -->|No| GW[1-Hop Graph Walk Query]
        GW -->|Join Counterparty History| PG[(PostgreSQL\ncompliance_decisions)]
        GW -->|Sanctioned Counterparty Found| F[Decision: FLAG\nRisk: 55]
        GW -->|Clean History| A[Decision: ALLOW\nRisk: 0]
    end

    subgraph Execution & Persistence
        B -->|Persist Audit Record| PG
        F -->|Persist Audit Record| PG
        A -->|Persist Audit Record| PG
        A -->|Submit Tx| ANVIL[Anvil EVM / Local Execution]
    end

    subgraph Telemetry & Regulatory Narration
        PG -->|Change Detection| API[Fastify API & WebSocket Gateway]
        API -->|Tx with Decision & Reason Codes| AI[AI Explainer Service\nFastAPI + Gemini 3.6 Flash]
        AI -->|Generate Regulatory Narrative| API
        API -->|Save Narrative| PG
        API -->|Live WS Broadcast| DASH[Next.js Mission Control Dashboard]
    end

    subgraph Post-Execution Attribution
        ANVIL -->|New Blocks| AW[Attribution Worker\nRust + Alloy]
        AW -->|Proposer Check| PG
        AW -->|Save Block Status| PG
    end
```

---

## 3-Tier Policy Decision Matrix

| Tier | Decision | Risk Score | Trigger Condition | Builder Action |
|---|---|---|---|---|
| **Tier 1** | **`BLOCK`** | **98** | Direct match in Redis cache against OFAC SDN or sanctioned entity list (`SANCTIONED_SENDER`, `SANCTIONED_RECIPIENT`). | Excluded immediately from block construction pre-execution. |
| **Tier 2** | **`FLAG`** | **55** | 1-hop audit graph walk reveals historical transaction with a directly sanctioned counterparty (`INDIRECT_SENDER_EXPOSURE`, `INDIRECT_RECIPIENT_EXPOSURE`). | Held for Enhanced Due Diligence (EDD) / compliance analyst review. |
| **Tier 3** | **`ALLOW`** | **0** | Address and counterparty history clean; zero sanctions exposure. | Included in candidate block for state execution. |

---

## Key Features

- **Sub-Millisecond Direct Lookups**: Direct sanctions checks run in $O(1)$ memory time via a Redis Set (`sanctioned_addresses`) bootstrapped on engine startup directly from authoritative attribution tables.
- **Audit-Trail Graph Walk**: Rather than relying on external heuristic graph databases, indirect risk is computed as a 1-hop SQL graph walk over the builder's own immutable `compliance_decisions` table.
- **Case-Insensitive EVM Address Handling**: All SQL joins and Redis lookups utilize lowercase normalization, ensuring mixed-case checksummed addresses (e.g. `0xAbC...`) match deterministically.
- **Strict Deterministic / AI Separation**: Policy decisions and risk scores are 100% computed by the compiled Rust engine. Gemini is invoked asynchronously solely to compose human-readable narratives for regulatory compliance logs.
- **Post-Execution Proposer Attribution**: A background worker monitors mined blocks on Anvil, checking validator coinbase addresses against `address_attributions` to flag blocks built by sanctioned proposers (`EXPOSED_EXTERNAL`).
- **Real-Time Mission Control Dashboard**: Next.js dashboard streaming live mempool activity, block builder telemetry, metrics counters, and detailed transaction lineage with AI explanations.

---

## Repository Structure

```
compliance-block-builder/
├── engine/              # High-performance Rust policy engine (Axum + SQLx + Redis)
├── simulator/           # Scenario runner (Alloy + Tokio) & block attribution worker
├── api/                 # Fastify orchestration layer with WebSocket streaming
├── ai-explainer/        # Python FastAPI microservice powered by Gemini 3.6 Flash
├── dashboard/           # Next.js / Tailwind CSS / Lucide real-time web dashboard
└── db/                  # PostgreSQL schema and sanctions entity seed scripts
```

---

## Quickstart Guide

### Prerequisites
- **Rust** (1.80+) & **Cargo**
- **Node.js** (v18+) & **npm**
- **Python** (3.11+) & **virtualenv**
- **PostgreSQL** & **Redis** installed locally
- **Foundry** (`anvil`) installed

---

### 1. Database & Sanctions Seeding

Create the database, apply the schema, and seed the authoritative OFAC sanctions attributions:

```bash
# Create PostgreSQL database
createdb compliance_builder

# Run schema and seed scripts
psql -d compliance_builder -f db/schema.sql
psql -d compliance_builder -f db/seed.sql
psql -d compliance_builder -f db/seed_addresses.sql
```

---

### 2. Environment Configuration

Copy the template environment files for each microservice:

```bash
# Engine (Rust)
cp engine/.env.example engine/.env

# API Orchestrator (Node.js)
cp api/.env.example api/.env

# AI Explainer (Python FastAPI)
cp ai-explainer/.env.example ai-explainer/.env
# Edit ai-explainer/.env and set your GEMINI_API_KEY from https://aistudio.google.com/

# Next.js Dashboard
cp dashboard/.env.example dashboard/.env.local
```

---

### 3. Start Core Infrastructure & Services

Open separate terminal windows for each component:

**Terminal 1: Local Ethereum Node (Anvil)**
```bash
anvil
```

**Terminal 2: Redis In-Memory Cache**
```bash
redis-server
```

**Terminal 3: Compliance Engine (Rust)**
```bash
cd engine
cargo run
# Bootstraps Redis Set: "Loaded 121 sanctioned addresses into Redis cache"
# Listening on http://127.0.0.1:3001
```

**Terminal 4: AI Explainer Microservice (Python)**
```bash
cd ai-explainer
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn main:app --port 8000 --reload
# Listening on http://127.0.0.1:8000
```

**Terminal 5: Fastify Orchestrator & WebSocket Gateway (Node.js)**
```bash
cd api
npm install
npm run dev
# Listening on http://localhost:3002
```

**Terminal 6: Next.js Telemetry Dashboard**
```bash
cd dashboard
npm install
npm run dev
# Running on http://localhost:3000
```

---

## Running the Demo Simulator

Execute the end-to-end multi-scenario simulation:

```bash
cd simulator
cargo run --bin simulator
```

The simulator runs three sequential scenarios with natural demo pacing:

1. **Scenario 1: Clean Transaction**
   - Clean sender $\to$ Clean recipient.
   - **Decision**: `ALLOW` (Risk: 0).
   - Submitted to Anvil and confirmed on-chain.

2. **Scenario 2: Directly Sanctioned Transaction**
   - Sender $\to$ OFAC Sanctioned recipient (`0x0330070FD38Ec3bB94F58FA55D40368271E9e54A`).
   - **Decision**: `BLOCK` (Risk: 98, Reason: `SANCTIONED_RECIPIENT`).
   - Blocked before submission to chain; audit record saved to database.

3. **Scenario 3: Indirect Exposure (1-Hop Graph Walk)**
   - That same sender $\to$ Clean recipient.
   - **Decision**: `FLAG` (Risk: 55, Reason: `INDIRECT_SENDER_EXPOSURE`).
   - Flagged for Enhanced Due Diligence because the sender transacted with a sanctioned entity in Scenario 2.

---

## Live Judge Verification & Inspection

Judges can inspect any layer of the running stack live to verify system claims:

### 1. Verify Redis Sanctions Cache
```bash
# Check count of preloaded OFAC-sanctioned addresses in memory:
redis-cli SCARD sanctioned_addresses
# Expected: 121

# Query direct hit in Redis:
redis-cli SISMEMBER sanctioned_addresses 0x0330070fd38ec3bb94f58fa55d40368271e9e54a
# Expected: 1
```

### 2. Verify Graph Walk Audit Records
```bash
psql -d compliance_builder -c "
SELECT tx_hash, sender, recipient, decision, risk_score, reason_codes, ai_explanation 
FROM compliance_decisions 
ORDER BY created_at DESC 
LIMIT 3;
"
```

### 3. Inspect Live AI Narration
Open the dashboard at `http://localhost:3000`, navigate to the **Lineage & Audit Inspector** tab, and click on any `BLOCK` or `FLAG` transaction to read the factual explanation generated by Gemini.
