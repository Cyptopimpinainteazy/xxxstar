use crate::models::{Module, Requirement, RequirementStatus, RiskLevel};

pub fn classify_module(text: &str, tags: &[String]) -> Module {
    let lower = text.to_lowercase();

    if has_tag(tags, "CONSENSUS")
        || has_any(
            &lower,
            &["consensus", "finality", "aura", "grandpa", "block"],
        )
    {
        return Module::Consensus;
    }
    if has_tag(tags, "CROSS_VM")
        || has_any(
            &lower,
            &[
                "cross-vm",
                "cross vm",
                "atomic",
                "coordinator",
                "dispatcher",
            ],
        )
    {
        return Module::CrossVm;
    }
    if has_tag(tags, "BRIDGE")
        || has_any(&lower, &["bridge", "external", "relayer", "proof", "nonce"])
    {
        return Module::Bridge;
    }
    if has_any(&lower, &["asset", "mint", "burn", "redeem", "wrapped"]) {
        return Module::UniversalAssetKernel;
    }
    if has_tag(tags, "DEX") || has_any(&lower, &["dex", "swap", "pool", "lp", "liquidity"]) {
        return Module::Dex;
    }
    if has_tag(tags, "GPU") || has_any(&lower, &["gpu", "cuda", "batch", "validator"]) {
        return Module::GpuValidator;
    }
    if has_any(&lower, &["wallet", "explorer", "faucet", "indexer"]) {
        return Module::WalletExplorer;
    }
    if has_tag(tags, "OPS") || has_any(&lower, &["testnet", "mainnet", "genesis", "monitoring"]) {
        return Module::LaunchOps;
    }
    if has_tag(tags, "SECURITY")
        || has_any(
            &lower,
            &["security", "replay", "signature", "attack", "invariant"],
        )
    {
        return Module::Security;
    }
    Module::Docs
}

pub fn classify_risk(req: &Requirement) -> RiskLevel {
    let text = req.text.to_lowercase();

    if req.status == RequirementStatus::Blocker {
        return RiskLevel::Critical;
    }

    if has_tag(&req.tags, "SECURITY")
        || has_any(
            &text,
            &["security", "exploit", "replay", "nonce", "critical"],
        )
    {
        return RiskLevel::Critical;
    }

    match req.module {
        Module::Consensus | Module::Bridge | Module::CrossVm => RiskLevel::High,
        Module::Dex | Module::GpuValidator | Module::UniversalAssetKernel => RiskLevel::Medium,
        _ => RiskLevel::Low,
    }
}

fn has_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn has_tag(tags: &[String], tag: &str) -> bool {
    tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_bridge_by_keywords() {
        let module = classify_module("Replay protection for bridge nonce", &[]);
        assert_eq!(module, Module::Bridge);
    }

    #[test]
    fn classifies_cross_vm_by_tag() {
        let module = classify_module("Some requirement", &["CROSS_VM".to_string()]);
        assert_eq!(module, Module::CrossVm);
    }
}
