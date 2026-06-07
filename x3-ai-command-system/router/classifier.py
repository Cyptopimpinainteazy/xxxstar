"""
classifier.py — X3 Model Router Classifier

Classifies user prompts against the model registry using priority-weighted
keyword matching with phrase detection. Falls back to cryptomaster when
no strong match is found.

Priority keywords are worth 5 points each (exact phrase match).
Standard keywords are worth 1-3 points depending on match quality.
Multi-keyword matches in the same prompt boost the score.
"""

import re
import yaml
from pathlib import Path
from typing import Optional


def load_registry(registry_path: Optional[str] = None) -> dict:
    """Load the model registry YAML."""
    if registry_path is None:
        registry_path = Path(__file__).parent / "model_registry.yaml"
    else:
        registry_path = Path(registry_path)
    with open(registry_path, "r") as f:
        return yaml.safe_load(f)


# Priority keywords that strongly indicate a specific model.
# These get a 10x weight bonus.
PRIORITY_KEYWORDS = {
    "solidity_guard": ["solidity", "foundry", "hardhat", "smart contract", "erc20", "erc721", "reentrancy"],
    "svm_guard": ["solana", "svm", "anchor program", "cpi", "spl token", "serum"],
    "cosmwasm_guard": ["cosmwasm", "cosmos sdk", "ibc transfer", "cw20", "cw721", "appchain"],
    "btc_guard": ["bitcoin", "btc", "utxo", "taproot", "psbt", "dlc"],
    "arb_king": ["arbitrage", "arb strategy", "dex arbitrage", "cross-chain arb", "triangular arb"],
    "flashloan_executor": ["flashloan", "flash loan", "aave flash", "dydx flash", "callback validation"],
    "route_oracle": ["route scoring", "route score", "finality risk", "atomicity class", "route verdict", "score this route", "score route"],
    "quant_risk": ["pnl model", "risk model", "volatility model", "circuit breaker", "max loss", "pnl dashboard", "pnl log", "risk score"],
    "trade_ops": ["trading bot", "scanner daemon", "trading infrastructure", "trade ops"],
    "mev_defense": ["mev protection", "sandwich defense", "flashbots", "private relay", "anti-sandwich", "mev defense"],
    "data_engineer": ["indexer", "etl pipeline", "price feed", "chain data", "data pipeline"],
    "devops_commander": ["docker", "systemd", "nginx", "deploy", "ci/cd", "monitoring setup", "infrastructure"],
    "testsmith": ["fuzz test", "property test", "invariant test", "test coverage", "test plan"],
    "docsmith": ["documentation", "readme", "model card", "api reference", "changelog"],
    "compliance_ops": ["grant application", "audit coordination", "compliance", "investor update", "regulation"],
    "eval_judge": ["eval", "score model", "model quality", "benchmark model", "regression"],
    "cline_finisher": ["todo", "fixme", "hack", "stub", "broken import", "failing test", "incomplete code", "finisher"],
    "rust_runtime": ["substrate", "pallet", "runtime", "x3vm", "dispatchable", "weight benchmark", "storage migration"],
    "auditor": ["audit", "security review", "mainnet readiness", "vulnerability", "exploit", "fund loss risk"],
    "cryptomaster": ["architecture", "planning", "cross-vm design", "mainnet readiness review"],
}


def normalize_prompt(prompt: str) -> str:
    """Normalize a prompt for keyword matching."""
    text = prompt.lower()
    text = re.sub(r'[^\w\s\-/\.]', ' ', text)
    text = re.sub(r'\s+', ' ', text).strip()
    return text


def classify_keyword(prompt: str, registry: dict) -> tuple:
    """
    Classify a prompt using priority-weighted keyword matching.

    Returns (model_key, ollama_model, reviewer_model, score).
    """
    normalized = normalize_prompt(prompt)
    models = registry.get("models", {})
    routing = registry.get("routing", {})

    scores = {}
    for model_key, model_info in models.items():
        handles = model_info.get("handles", [])
        score = 0

        # Check priority keywords (10x weight)
        priority_kw = PRIORITY_KEYWORDS.get(model_key, [])
        for kw in priority_kw:
            if kw in normalized:
                score += 10

        # Check registry handles (standard weight)
        for keyword in handles:
            kw_lower = keyword.lower()
            # Exact phrase match in prompt
            if kw_lower in normalized:
                # Longer keywords are more specific
                word_count = len(kw_lower.split())
                weight = min(word_count + 1, 5)
                score += weight
            # Partial match (keyword is substring of a word)
            elif any(kw_lower in word for word in normalized.split()):
                score += 1

        if score > 0:
            scores[model_key] = score

    # Find the best match
    if not scores:
        best_key = "cryptomaster"
        best_score = 0
    else:
        # Sort by score descending, then by specificity (fewer handles = more specific)
        best_key = max(scores, key=lambda k: (scores[k], -len(models[k].get("handles", []))))
        best_score = scores[best_key]

    # Look up routing for reviewer
    route_info = None
    for route_key, route_data in routing.items():
        if route_data.get("primary") == best_key:
            route_info = route_data
            break

    reviewer_key = route_info.get("reviewer", "auditor") if route_info else "auditor"

    primary_model = models.get(best_key, models["cryptomaster"]).get("ollama", f"lojak/{best_key}")
    reviewer_model = models.get(reviewer_key, models.get("auditor", {})).get("ollama", "lojak/x3-auditor")

    return best_key, primary_model, reviewer_model, best_score


def classify(prompt: str, registry_path: Optional[str] = None, mode: str = "keyword") -> tuple:
    """Classify a prompt and return (model_key, ollama_model, reviewer_model, score)."""
    registry = load_registry(registry_path)
    if mode == "keyword":
        return classify_keyword(prompt, registry)
    else:
        raise NotImplementedError(f"Classification mode '{mode}' not yet implemented. Use 'keyword'.")


if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print("Usage: python classifier.py <prompt>")
        print("       python classifier.py --test")
        sys.exit(1)

    if sys.argv[1] == "--test":
        test_cases = [
            ("Audit this Solidity contract for reentrancy", "solidity_guard"),
            ("Fix the failing tests in the Rust pallet", "rust_runtime"),
            ("Design an arbitrage strategy for cross-chain DEX", "arb_king"),
            ("Build a flashloan executor for Aave V3", "flashloan_executor"),
            ("Score this route: Uniswap to Raydium via Wormhole", "route_oracle"),
            ("Review the mainnet readiness of X3 Chain", "auditor"),
            ("Write a Solana program for token swap", "svm_guard"),
            ("Set up Docker and systemd for the validator node", "devops_commander"),
            ("Kill all TODOs and fix broken imports", "cline_finisher"),
            ("Hello, how are you today?", "cryptomaster"),
            ("Implement the BTC to X3 bridge custody proof", "btc_guard"),
            ("Write Foundry tests for the vault contract", "solidity_guard"),
            ("Check MEV exposure on this trading route", "mev_defense"),
            ("Build an indexer for X3 events", "data_engineer"),
            ("Calculate the risk model for this arb strategy", "quant_risk"),
            ("Set up CI/CD pipeline for deployment", "devops_commander"),
            ("Write the grant application for X3", "compliance_ops"),
            ("Write a CosmWasm IBC transfer handler", "cosmwasm_guard"),
            ("Design the PnL dashboard for trading ops", "quant_risk"),
            ("Build a Taproot script for the bridge", "btc_guard"),
            ("Score model outputs for regression", "eval_judge"),
            ("Update the README and model card", "docsmith"),
            ("Design the X3 cross-VM architecture", "cryptomaster"),
            ("Audit the mainnet readiness checklist", "auditor"),
        ]

        print("X3 Router Classifier Tests")
        print("=" * 80)

        passed = 0
        failed = 0
        for prompt, expected in test_cases:
            key, model, reviewer, score = classify(prompt)
            status = "✓" if key == expected else "✗"
            if key == expected:
                passed += 1
            else:
                failed += 1
            print(f"  {status}  Score={score:3d}  Got={key:25s}  Expected={expected:25s}")
            if key != expected:
                print(f"       Prompt: {prompt[:70]}")

        print()
        print(f"Results: {passed}/{len(test_cases)} passed, {failed}/{len(test_cases)} failed")
        sys.exit(0 if failed == 0 else 1)

    prompt = " ".join(sys.argv[1:])
    key, model, reviewer, score = classify(prompt)
    print(f"Prompt:     {prompt}")
    print(f"Model key:  {key}")
    print(f"Ollama:     {model}")
    print(f"Reviewer:   {reviewer}")
    print(f"Score:      {score}")