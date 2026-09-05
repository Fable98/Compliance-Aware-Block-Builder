# Compliance-Aware-Block-Builder
A compliance-aware block builder that screens every transaction against OFAC/sanctions data before execution. Rust engine makes deterministic ALLOW/REVIEW/BLOCK decisions, revm simulates ALLOW transactions against Anvil's live state, and a Python/FastAPI service narrates the outcome after the fact — the LLM explains decisions, it never makes them.
