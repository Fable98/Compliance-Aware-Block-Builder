from fastapi import FastAPI
from pydantic import BaseModel
from typing import List
import os
from dotenv import load_dotenv
from google import genai

load_dotenv()

app = FastAPI(title="Compliance AI Explainer")

client = genai.Client(api_key=os.getenv("GEMINI_API_KEY"))


class DecisionInput(BaseModel):
    tx: str
    decision: str
    risk_score: int
    reasons: List[str]


class ExplanationOutput(BaseModel):
    tx: str
    decision: str
    risk_score: int
    narrative: str


@app.post("/explain", response_model=ExplanationOutput)
def explain_decision(input: DecisionInput):
    prompt = f"""You are a compliance narration assistant for a blockchain transaction screening system.
You NEVER make decisions — a deterministic Rust policy engine already decided. Your only job is to
explain the decision that was already made, in one or two clear sentences suitable for a compliance
audit report.

Deterministic engine output:
- Transaction: {input.tx}
- Decision: {input.decision}
- Risk Score: {input.risk_score}
- Reason Codes: {', '.join(input.reasons) if input.reasons else 'None'}

Write a short, factual, professional explanation of why this decision was made, referencing the
reason codes. Do not speculate beyond what the reason codes indicate. Do not suggest any action —
only explain what happened and why."""

    response = client.models.generate_content(
        model="gemini-3.6-flash",
        contents=prompt,
    )

    narrative = response.text

    return ExplanationOutput(
        tx=input.tx,
        decision=input.decision,
        risk_score=input.risk_score,
        narrative=narrative,
    )


@app.get("/health")
def health():
    return {"status": "ok"}
