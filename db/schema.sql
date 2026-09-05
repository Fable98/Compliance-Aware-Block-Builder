CREATE TABLE sanctions_entities (
    entity_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    source VARCHAR(50) NOT NULL,
    list_type VARCHAR(50) NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE address_attributions (
    address VARCHAR(42) PRIMARY KEY,
    entity_id UUID REFERENCES sanctions_entities(entity_id),
    attribution_type VARCHAR(50),
    confidence NUMERIC(3, 2) NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE compliance_decisions (
    tx_hash VARCHAR(66) PRIMARY KEY,
    sender VARCHAR(42) NOT NULL,
    recipient VARCHAR(42) NOT NULL,
    decision VARCHAR(10) NOT NULL,
    risk_score INT NOT NULL,
    reason_codes TEXT[] NOT NULL,
    ai_explanation TEXT,
    policy_version VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE blocks (
    block_hash VARCHAR(66) PRIMARY KEY,
    block_number BIGINT NOT NULL,
    builder_address VARCHAR(42) NOT NULL,
    proposer_entity_id UUID REFERENCES sanctions_entities(entity_id),
    tx_count INT NOT NULL,
    compliance_status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_compliance_decisions_created_at ON compliance_decisions(created_at DESC);
CREATE INDEX idx_blocks_block_number ON blocks(block_number DESC);
CREATE INDEX idx_address_attributions_entity ON address_attributions(entity_id);
