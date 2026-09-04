use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

use crate::models::{
    FrontendRouteAllowlist, FrontendRouteAllowlistEntry, RpcConsumerContractEntry,
    RpcConsumerContracts, RpcContractBucket, RpcContractFlag, RpcContractFlagCategory,
    RpcContractFlagSeverity, RpcContractMatrix, RpcMethodContract, RuntimeApiMethodInventory,
    RuntimeApiTraitInventory, RuntimeRpcInventory, SidecarAdapterBacklog,
    SidecarAdapterBacklogEntry,
};

const RUNTIME_SOURCE: &str = "runtime/src/lib.rs";
const RPC_SOURCE: &str = "node/src/rpc.rs";

pub struct InventoryOutputs {
    pub runtime_inventory: RuntimeRpcInventory,
    pub contract_matrix: RpcContractMatrix,
    pub contract_matrix_md: String,
    pub consumer_contracts: RpcConsumerContracts,
    pub consumer_contracts_md: String,
    pub frontend_route_allowlist: FrontendRouteAllowlist,
    pub frontend_route_allowlist_md: String,
    pub sidecar_adapter_backlog: SidecarAdapterBacklog,
    pub sidecar_adapter_backlog_md: String,
}

struct ParsedRpcRegistration {
    method: RpcMethodContract,
    branch_kind: Option<BranchKind>,
    branch_condition: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BranchKind {
    If,
    Else,
}

pub fn generate_inventory(root: &Path, generated_at: &str) -> Result<InventoryOutputs> {
    let runtime_path = root.join(RUNTIME_SOURCE);
    let rpc_path = root.join(RPC_SOURCE);

    let runtime_traits = parse_runtime_traits(&runtime_path)?;
    let method_to_traits = build_method_to_traits(&runtime_traits);
    let parsed_methods = parse_rpc_methods(&rpc_path, &method_to_traits)?;
    let methods: Vec<RpcMethodContract> = parsed_methods
        .iter()
        .map(|item| item.method.clone())
        .collect();
    let flags = build_contract_flags(&parsed_methods);
    let duplicate_registration_count = flags
        .iter()
        .filter(|item| item.category == RpcContractFlagCategory::DuplicateRegistration)
        .count();
    let bucket_drift_count = flags
        .iter()
        .filter(|item| item.category == RpcContractFlagCategory::BucketDrift)
        .count();

    let runtime_backed_count = methods
        .iter()
        .filter(|item| matches!(item.bucket, RpcContractBucket::RuntimeBacked))
        .count();
    let node_local_adapter_count = methods
        .iter()
        .filter(|item| matches!(item.bucket, RpcContractBucket::NodeLocalAdapter))
        .count();
    let placeholder_count = methods
        .iter()
        .filter(|item| matches!(item.bucket, RpcContractBucket::Placeholder))
        .count();

    let runtime_inventory = RuntimeRpcInventory {
        generated_at: generated_at.to_string(),
        runtime_source_file: RUNTIME_SOURCE.to_string(),
        rpc_source_file: RPC_SOURCE.to_string(),
        runtime_traits,
    };

    let contract_matrix = RpcContractMatrix {
        generated_at: generated_at.to_string(),
        rpc_source_file: RPC_SOURCE.to_string(),
        runtime_backed_count,
        node_local_adapter_count,
        placeholder_count,
        duplicate_registration_count,
        bucket_drift_count,
        flags,
        methods,
    };

    let contract_matrix_md = render_contract_matrix_markdown(&contract_matrix);
    let consumer_contracts = build_consumer_contracts(generated_at, &parsed_methods);
    let consumer_contracts_md = render_consumer_contracts_markdown(&consumer_contracts);
    let frontend_route_allowlist =
        build_frontend_route_allowlist(generated_at, &consumer_contracts);
    let frontend_route_allowlist_md =
        render_frontend_route_allowlist_markdown(&frontend_route_allowlist);
    let sidecar_adapter_backlog = build_sidecar_adapter_backlog(generated_at, &consumer_contracts);
    let sidecar_adapter_backlog_md =
        render_sidecar_adapter_backlog_markdown(&sidecar_adapter_backlog);

    Ok(InventoryOutputs {
        runtime_inventory,
        contract_matrix,
        contract_matrix_md,
        consumer_contracts,
        consumer_contracts_md,
        frontend_route_allowlist,
        frontend_route_allowlist_md,
        sidecar_adapter_backlog,
        sidecar_adapter_backlog_md,
    })
}

fn build_contract_flags(methods: &[ParsedRpcRegistration]) -> Vec<RpcContractFlag> {
    let mut grouped: BTreeMap<&str, Vec<&ParsedRpcRegistration>> = BTreeMap::new();
    for method in methods {
        grouped
            .entry(method.method.method.as_str())
            .or_default()
            .push(method);
    }

    let mut flags = Vec::new();
    for (method_name, entries) in grouped {
        if entries.len() > 1 && !is_expected_conditional_duplicate(&entries) {
            flags.push(RpcContractFlag {
                category: RpcContractFlagCategory::DuplicateRegistration,
                severity: RpcContractFlagSeverity::High,
                method: method_name.to_string(),
                line_refs: entries
                    .iter()
                    .map(|entry| format!("{}:{}", entry.method.source_file, entry.method.line))
                    .collect(),
                reason: format!(
                    "method is registered {} times; consumer contracts must treat this as a drift risk until the duplicate registrations are collapsed",
                    entries.len()
                ),
            });
        }

        if entries.iter().any(|entry| {
            !entry.method.runtime_calls.is_empty()
                && !entry.method.node_local_signals.is_empty()
                && entry.method.ownership_note.is_none()
        }) {
            flags.push(RpcContractFlag {
                category: RpcContractFlagCategory::BucketDrift,
                severity: RpcContractFlagSeverity::Warn,
                method: method_name.to_string(),
                line_refs: entries
                    .iter()
                    .filter(|entry| {
                        !entry.method.runtime_calls.is_empty()
                            && !entry.method.node_local_signals.is_empty()
                            && entry.method.ownership_note.is_none()
                    })
                    .map(|entry| format!("{}:{}", entry.method.source_file, entry.method.line))
                    .collect(),
                reason: "method mixes runtime API calls with node-local signals; treat it as sidecar-owned until the ownership boundary is explicit".to_string(),
            });
        }
    }

    flags
}

fn is_expected_conditional_duplicate(entries: &[&ParsedRpcRegistration]) -> bool {
    if entries.len() != 2 {
        return false;
    }

    let first = entries[0];
    let second = entries[1];
    matches!(first.branch_kind, Some(BranchKind::If))
        && matches!(second.branch_kind, Some(BranchKind::Else))
        && first.branch_condition.is_some()
        && first.branch_condition == second.branch_condition
}

fn detect_expected_mixed_ownership(
    method_name: &str,
    runtime_calls: &BTreeSet<String>,
    node_local_signals: &[String],
) -> Option<String> {
    if runtime_calls.is_empty() || node_local_signals.is_empty() {
        return None;
    }

    match method_name {
        "x3_submitCrossVmTransaction" => Some(
            "expected mixed ownership: runtime executes the EVM leg while node-local billing and queueing own the orchestration boundary".to_string(),
        ),
        _ => None,
    }
}

fn build_consumer_contracts(
    generated_at: &str,
    methods: &[ParsedRpcRegistration],
) -> RpcConsumerContracts {
    let collapsed = collapse_methods_for_consumers(methods);

    let mut frontend_safe_methods = Vec::new();
    let mut sidecar_only_methods = Vec::new();
    let mut mock_only_methods = Vec::new();

    for entry in collapsed {
        match entry.frontend_consumer_mode.as_str() {
            "direct_read_candidate" => frontend_safe_methods.push(entry),
            "mock_only" => mock_only_methods.push(entry),
            _ => sidecar_only_methods.push(entry),
        }
    }

    RpcConsumerContracts {
        generated_at: generated_at.to_string(),
        source_matrix_file: "rpc_contract_matrix.json".to_string(),
        frontend_safe_count: frontend_safe_methods.len(),
        sidecar_only_count: sidecar_only_methods.len(),
        mock_only_count: mock_only_methods.len(),
        frontend_safe_methods,
        sidecar_only_methods,
        mock_only_methods,
    }
}

fn collapse_methods_for_consumers(
    methods: &[ParsedRpcRegistration],
) -> Vec<RpcConsumerContractEntry> {
    let mut grouped: BTreeMap<&str, Vec<&ParsedRpcRegistration>> = BTreeMap::new();
    for method in methods {
        grouped
            .entry(method.method.method.as_str())
            .or_default()
            .push(method);
    }

    let mut entries = Vec::new();
    for (method_name, registrations) in grouped {
        let mut runtime_trait_hints = BTreeSet::new();
        let mut node_local_signals = BTreeSet::new();
        let mut notes = BTreeSet::new();
        let mut selected_bucket = RpcContractBucket::RuntimeBacked;
        let mut frontend_mode = "direct_read_candidate";
        let mut sidecar_mode = "pass_through_candidate";

        for registration in &registrations {
            if bucket_rank(&registration.method.bucket) > bucket_rank(&selected_bucket) {
                selected_bucket = registration.method.bucket;
            }
            if frontend_rank(&registration.method.frontend_consumer_mode)
                > frontend_rank(frontend_mode)
            {
                frontend_mode = &registration.method.frontend_consumer_mode;
            }
            if sidecar_rank(&registration.method.sidecar_consumer_mode) > sidecar_rank(sidecar_mode)
            {
                sidecar_mode = &registration.method.sidecar_consumer_mode;
            }
            runtime_trait_hints.extend(registration.method.runtime_trait_hints.iter().cloned());
            node_local_signals.extend(registration.method.node_local_signals.iter().cloned());
            if let Some(reason) = &registration.method.placeholder_reason {
                notes.insert(reason.clone());
            }
        }

        if registrations.len() > 1 && !is_expected_conditional_duplicate(&registrations) {
            notes.insert(format!(
                "duplicate registrations detected; consumer contract was collapsed conservatively from {} registrations",
                registrations.len()
            ));
        }
        if !node_local_signals.is_empty() && !runtime_trait_hints.is_empty() {
            notes.insert(
                "contains both runtime-backed and node-local behavior; keep frontend integration behind sidecar or adapter ownership".to_string(),
            );
        }
        if notes.is_empty() {
            notes.insert("-".to_string());
        }

        entries.push(RpcConsumerContractEntry {
            method: method_name.to_string(),
            registration_count: registrations.len(),
            bucket: selected_bucket,
            frontend_consumer_mode: frontend_mode.to_string(),
            sidecar_consumer_mode: sidecar_mode.to_string(),
            ownership_note: registrations
                .iter()
                .find_map(|entry| entry.method.ownership_note.clone()),
            runtime_trait_hints: runtime_trait_hints.into_iter().collect(),
            node_local_signals: node_local_signals.into_iter().collect(),
            notes: notes.into_iter().collect(),
        });
    }

    entries
}

fn bucket_rank(bucket: &RpcContractBucket) -> u8 {
    match bucket {
        RpcContractBucket::RuntimeBacked => 0,
        RpcContractBucket::NodeLocalAdapter => 1,
        RpcContractBucket::Placeholder => 2,
    }
}

fn frontend_rank(mode: &str) -> u8 {
    match mode {
        "direct_read_candidate" => 0,
        "adapter_only" => 1,
        "mock_only" => 2,
        _ => 3,
    }
}

fn sidecar_rank(mode: &str) -> u8 {
    match mode {
        "pass_through_candidate" => 0,
        "orchestrate" => 1,
        "defer" => 2,
        _ => 3,
    }
}

fn parse_runtime_traits(path: &Path) -> Result<Vec<RuntimeApiTraitInventory>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let impl_re = Regex::new(r"^\s*impl\s+(.+?)\s+for\s+Runtime\s*\{")?;
    let fn_re = Regex::new(r"^\s*fn\s+([A-Za-z0-9_]+)\s*\(")?;

    let mut items = Vec::new();
    let mut pending_cfg: Option<String> = None;
    let mut idx = 0usize;
    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(") {
            pending_cfg = Some(trimmed.to_string());
            idx += 1;
            continue;
        }

        if let Some(caps) = impl_re.captures(line) {
            let trait_name = normalize_ws(caps.get(1).map(|m| m.as_str()).unwrap_or_default());
            let start_line = idx + 1;
            let cfg_guard = pending_cfg.take();
            let mut methods = Vec::new();

            let mut brace_depth = brace_delta(line);
            let mut inner = idx + 1;
            while inner < lines.len() && brace_depth > 0 {
                let current = lines[inner];
                if brace_depth == 1 {
                    if let Some(method_caps) = fn_re.captures(current) {
                        methods.push(RuntimeApiMethodInventory {
                            name: method_caps
                                .get(1)
                                .map(|m| m.as_str().to_string())
                                .unwrap_or_default(),
                            line: inner + 1,
                        });
                    }
                }
                brace_depth += brace_delta(current);
                inner += 1;
            }

            items.push(RuntimeApiTraitInventory {
                trait_name,
                source_file: RUNTIME_SOURCE.to_string(),
                impl_line: start_line,
                cfg_guard,
                methods,
            });
            idx = inner;
            continue;
        }

        if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("#") {
            pending_cfg = None;
        }
        idx += 1;
    }

    Ok(items)
}

fn parse_rpc_methods(
    path: &Path,
    method_to_traits: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<ParsedRpcRegistration>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let register_re = Regex::new(r#"register_method\("([^"]+)""#)?;
    let runtime_call_re = Regex::new(r"(?s)(?:api|runtime_api)\s*\.\s*([A-Za-z0-9_]+)\s*\(")?;

    let mut starts = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if let Some(caps) = register_re.captures(line) {
            starts.push((
                idx,
                caps.get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            ));
        }
    }

    let mut methods = Vec::new();
    for (start_idx, method_name) in starts.iter() {
        let end_idx = registration_end_index(&lines, *start_idx);
        let chunk = lines[*start_idx..end_idx].join("\n");
        let (branch_kind, branch_condition) = registration_branch_context(&lines, *start_idx);

        let mut runtime_calls = BTreeSet::new();
        for caps in runtime_call_re.captures_iter(&chunk) {
            if let Some(name) = caps.get(1) {
                runtime_calls.insert(name.as_str().to_string());
            }
        }

        let mut runtime_trait_hints = BTreeSet::new();
        for call in &runtime_calls {
            if let Some(traits) = method_to_traits.get(call) {
                for trait_name in traits {
                    runtime_trait_hints.insert(trait_name.clone());
                }
            }
        }

        let node_local_signals = detect_node_local_signals(&chunk);
        let placeholder_reason = detect_placeholder_reason(method_name, &chunk);
        let bucket = classify_bucket(
            &runtime_calls,
            &node_local_signals,
            placeholder_reason.as_deref(),
        );
        let ownership_note =
            detect_expected_mixed_ownership(method_name, &runtime_calls, &node_local_signals);

        methods.push(ParsedRpcRegistration {
            method: RpcMethodContract {
                method: method_name.clone(),
                source_file: RPC_SOURCE.to_string(),
                line: *start_idx + 1,
                bucket,
                runtime_calls: runtime_calls.into_iter().collect(),
                runtime_trait_hints: runtime_trait_hints.into_iter().collect(),
                node_local_signals,
                placeholder_reason,
                ownership_note,
                frontend_consumer_mode: frontend_mode(method_name, &bucket).to_string(),
                sidecar_consumer_mode: sidecar_mode(method_name, &bucket).to_string(),
            },
            branch_kind,
            branch_condition,
        });
    }

    Ok(methods)
}

fn registration_end_index(lines: &[&str], start_idx: usize) -> usize {
    let mut paren_depth = 0i32;
    let mut seen_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        for ch in line.chars() {
            if ch == '(' {
                paren_depth += 1;
                seen_open = true;
            } else if ch == ')' {
                paren_depth -= 1;
            }
        }
        if seen_open && paren_depth <= 0 && line.trim_end().ends_with("?;") {
            return idx + 1;
        }
    }
    lines.len()
}

fn registration_branch_context(
    lines: &[&str],
    start_idx: usize,
) -> (Option<BranchKind>, Option<String>) {
    let current_depth = depth_before_line(lines, start_idx);
    for idx in (0..start_idx).rev() {
        let trimmed = lines[idx].trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let line_depth = depth_before_line(lines, idx);
        if trimmed == "} else {" && line_depth == current_depth {
            return (
                Some(BranchKind::Else),
                find_previous_if_condition(lines, idx, current_depth - 1),
            );
        }
        if trimmed.ends_with("else {") && line_depth + 1 == current_depth {
            return (
                Some(BranchKind::Else),
                find_previous_if_condition(lines, idx, line_depth),
            );
        }
        if (trimmed.starts_with("if ") || trimmed.starts_with("if let "))
            && line_depth + 1 == current_depth
        {
            return (
                Some(BranchKind::If),
                Some(normalize_ws(trimmed.trim_end_matches('{').trim())),
            );
        }
    }
    (None, None)
}

fn find_previous_if_condition(
    lines: &[&str],
    else_idx: usize,
    expected_depth: i32,
) -> Option<String> {
    for idx in (0..else_idx).rev() {
        let trimmed = lines[idx].trim();
        if (trimmed.starts_with("if ") || trimmed.starts_with("if let "))
            && depth_before_line(lines, idx) == expected_depth
        {
            return Some(normalize_ws(trimmed.trim_end_matches('{').trim()));
        }
    }
    None
}

fn depth_before_line(lines: &[&str], line_idx: usize) -> i32 {
    let mut depth = 0i32;
    for line in lines.iter().take(line_idx) {
        depth += brace_delta(line);
    }
    depth
}

fn build_method_to_traits(traits: &[RuntimeApiTraitInventory]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in traits {
        for method in &item.methods {
            map.entry(method.name.clone())
                .or_default()
                .push(item.trait_name.clone());
        }
    }
    for values in map.values_mut() {
        values.sort();
        values.dedup();
    }
    map
}

fn detect_node_local_signals(chunk: &str) -> Vec<String> {
    let signals = [
        ("wallet_dex", "WalletDexRpc service adapter"),
        ("swap_rpc", "SwapRPCServer node-local swap engine"),
        ("cross_vm_bridge", "CrossVmBridge node-local queue"),
        ("billing_check", "BillingMiddleware quota path"),
        ("billing_guard", "BillingMiddleware quota path"),
        ("flash_finality_gadget", "flash finality gadget"),
        ("g.metrics()", "flash finality gadget"),
        ("tokio::", "async node-local orchestration"),
    ];

    let mut found = BTreeSet::new();
    for (needle, label) in signals {
        if chunk.contains(needle) {
            found.insert(label.to_string());
        }
    }
    found.into_iter().collect()
}

fn detect_placeholder_reason(method_name: &str, chunk: &str) -> Option<String> {
    if chunk.contains("not available on this node build") {
        return Some("not available on this node build".to_string());
    }
    if method_name == "x3_submitX3vmTransaction"
        || chunk.contains("Use x3_submitCrossVmTransaction with Comit v2 payloads")
    {
        return Some(
            "intentional guidance error; standalone X3VM submission is unavailable on this build"
                .to_string(),
        );
    }
    if chunk.contains("Structural validation only") || chunk.contains("TODO(Phase 10b)") {
        return Some(
            "structural placeholder until Phase 10b challenge execution is implemented".to_string(),
        );
    }
    None
}

fn classify_bucket(
    runtime_calls: &BTreeSet<String>,
    node_local_signals: &[String],
    placeholder_reason: Option<&str>,
) -> RpcContractBucket {
    if placeholder_reason.is_some() {
        RpcContractBucket::Placeholder
    } else if runtime_calls.is_empty() {
        RpcContractBucket::NodeLocalAdapter
    } else if node_local_signals.is_empty() {
        RpcContractBucket::RuntimeBacked
    } else {
        RpcContractBucket::NodeLocalAdapter
    }
}

fn frontend_mode(method_name: &str, bucket: &RpcContractBucket) -> &'static str {
    match bucket {
        RpcContractBucket::Placeholder => "mock_only",
        RpcContractBucket::RuntimeBacked => {
            if is_read_like(method_name) {
                "direct_read_candidate"
            } else {
                "adapter_only"
            }
        }
        RpcContractBucket::NodeLocalAdapter => "adapter_only",
    }
}

fn sidecar_mode(method_name: &str, bucket: &RpcContractBucket) -> &'static str {
    match bucket {
        RpcContractBucket::Placeholder => "defer",
        RpcContractBucket::RuntimeBacked => {
            if is_read_like(method_name) {
                "pass_through_candidate"
            } else {
                "orchestrate"
            }
        }
        RpcContractBucket::NodeLocalAdapter => "orchestrate",
    }
}

fn is_read_like(method_name: &str) -> bool {
    method_name.starts_with("x3_get")
        || method_name.starts_with("query")
        || method_name.starts_with("validate")
        || method_name == "system_accountNextIndex"
        || method_name == "payment_queryInfo"
        || method_name == "gpu_orchestratorHealth"
        || method_name == "gpu_validatorStatus"
        || method_name == "x3_flashFinalityStatus"
}

fn render_contract_matrix_markdown(matrix: &RpcContractMatrix) -> String {
    let mut out = String::new();
    out.push_str("# Generated RPC Contract Matrix\n\n");
    out.push_str("This file is generated from live code by LaunchOps. Do not hand-edit it.\n\n");
    out.push_str(&format!("Generated at: {}\n\n", matrix.generated_at));
    out.push_str(&format!("Source: `{}`\n\n", matrix.rpc_source_file));
    out.push_str(&format!(
        "Buckets: runtime_backed={} node_local_adapter={} placeholder={}\n\n",
        matrix.runtime_backed_count, matrix.node_local_adapter_count, matrix.placeholder_count
    ));
    out.push_str(&format!(
        "Flags: duplicate_registrations={} bucket_drift={}\n\n",
        matrix.duplicate_registration_count, matrix.bucket_drift_count
    ));

    if !matrix.flags.is_empty() {
        out.push_str("## Flags\n\n");
        out.push_str("| Category | Severity | Method | Lines | Reason |\n");
        out.push_str("| --- | --- | --- | --- | --- |\n");
        for flag in &matrix.flags {
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | `{}` | {} |\n",
                serde_json::to_string(&flag.category)
                    .unwrap_or_else(|_| "\"unknown\"".to_string())
                    .trim_matches('"'),
                serde_json::to_string(&flag.severity)
                    .unwrap_or_else(|_| "\"unknown\"".to_string())
                    .trim_matches('"'),
                flag.method,
                flag.line_refs.join(", "),
                flag.reason
            ));
        }
        out.push('\n');
    }

    for bucket in [
        RpcContractBucket::RuntimeBacked,
        RpcContractBucket::NodeLocalAdapter,
        RpcContractBucket::Placeholder,
    ] {
        let heading = match bucket {
            RpcContractBucket::RuntimeBacked => "## Runtime-Backed",
            RpcContractBucket::NodeLocalAdapter => "## Node-Local Adapter",
            RpcContractBucket::Placeholder => "## Placeholder",
        };
        out.push_str(heading);
        out.push_str("\n\n");
        out.push_str("| Method | Runtime Calls | Trait Hints | Frontend | Sidecar | Notes |\n");
        out.push_str("| --- | --- | --- | --- | --- | --- |\n");

        for method in matrix.methods.iter().filter(|item| item.bucket == bucket) {
            let runtime_calls = if method.runtime_calls.is_empty() {
                "-".to_string()
            } else {
                method.runtime_calls.join(", ")
            };
            let trait_hints = if method.runtime_trait_hints.is_empty() {
                "-".to_string()
            } else {
                method.runtime_trait_hints.join(", ")
            };
            let notes = if let Some(reason) = &method.placeholder_reason {
                reason.clone()
            } else if let Some(note) = &method.ownership_note {
                note.clone()
            } else if !method.node_local_signals.is_empty() {
                method.node_local_signals.join(", ")
            } else {
                "-".to_string()
            };

            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | `{}` | `{}` | {} |\n",
                method.method,
                runtime_calls,
                trait_hints,
                method.frontend_consumer_mode,
                method.sidecar_consumer_mode,
                notes
            ));
        }
        out.push('\n');
    }

    out
}

fn render_consumer_contracts_markdown(contracts: &RpcConsumerContracts) -> String {
    let mut out = String::new();
    out.push_str("# Generated RPC Consumer Contracts\n\n");
    out.push_str(
        "This file is generated from the LaunchOps RPC contract matrix. Do not hand-edit it.\n\n",
    );
    out.push_str(&format!("Generated at: {}\n\n", contracts.generated_at));
    out.push_str(&format!(
        "Source matrix: `{}`\n\n",
        contracts.source_matrix_file
    ));
    out.push_str(&format!(
        "Contract split: frontend_safe={} sidecar_only={} mock_only={}\n\n",
        contracts.frontend_safe_count, contracts.sidecar_only_count, contracts.mock_only_count
    ));

    render_consumer_section(
        &mut out,
        "## Frontend-Safe",
        &contracts.frontend_safe_methods,
    );
    render_consumer_section(&mut out, "## Sidecar-Only", &contracts.sidecar_only_methods);
    render_consumer_section(&mut out, "## Mock-Only", &contracts.mock_only_methods);

    out
}

fn render_consumer_section(out: &mut String, heading: &str, entries: &[RpcConsumerContractEntry]) {
    out.push_str(heading);
    out.push_str("\n\n");
    out.push_str("| Method | Registrations | Bucket | Frontend | Sidecar | Trait Hints | Ownership | Notes |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for entry in entries {
        let trait_hints = if entry.runtime_trait_hints.is_empty() {
            "-".to_string()
        } else {
            entry.runtime_trait_hints.join(", ")
        };
        let ownership = entry.ownership_note.as_deref().unwrap_or("-");
        let notes = if entry.notes.is_empty() {
            "-".to_string()
        } else {
            entry.notes.join("; ")
        };
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} |\n",
            entry.method,
            entry.registration_count,
            serde_json::to_string(&entry.bucket)
                .unwrap_or_else(|_| "\"unknown\"".to_string())
                .trim_matches('"'),
            entry.frontend_consumer_mode,
            entry.sidecar_consumer_mode,
            trait_hints,
            ownership,
            notes
        ));
    }
    out.push('\n');
}

fn build_frontend_route_allowlist(
    generated_at: &str,
    contracts: &RpcConsumerContracts,
) -> FrontendRouteAllowlist {
    let mut routes = vec![
        FrontendRouteAllowlistEntry {
            route_id: "wallet-home".to_string(),
            route_label: "Wallet Home".to_string(),
            rationale: "Direct-read wallet and account posture data only; no relayer, queue, or signer-owned mutations.".to_string(),
            allowed_methods: filter_methods(
                &contracts.frontend_safe_methods,
                &["x3_getAssetMetadata", "x3_getAuthorities", "x3_getAuthorizedAccounts", "x3_getCanonicalBalance"],
            ),
        },
        FrontendRouteAllowlistEntry {
            route_id: "network-overview".to_string(),
            route_label: "Network Overview".to_string(),
            rationale: "Public network posture can bind to direct-read validator and operational health endpoints only.".to_string(),
            allowed_methods: filter_methods(
                &contracts.frontend_safe_methods,
                &["gpu_orchestratorHealth", "gpu_validatorStatus"],
            ),
        },
        FrontendRouteAllowlistEntry {
            route_id: "bridge-status".to_string(),
            route_label: "Bridge Status".to_string(),
            rationale: "Only read-side proof and validation signals are allowed directly; settlement and cross-VM submission remain sidecar-owned.".to_string(),
            allowed_methods: filter_methods(
                &contracts.frontend_safe_methods,
                &["query_crossChainStatus", "validate_evmHeader", "validate_svmHeader"],
            ),
        },
        FrontendRouteAllowlistEntry {
            route_id: "governance".to_string(),
            route_label: "Governance Desk".to_string(),
            rationale: "Direct-read governance dispute and finality visibility is allowed; proposal actions stay behind sidecar and signing boundaries.".to_string(),
            allowed_methods: filter_methods(
                &contracts.frontend_safe_methods,
                &["queryDisputeStatus", "queryProofFinality"],
            ),
        },
        FrontendRouteAllowlistEntry {
            route_id: "explorer".to_string(),
            route_label: "Explorer Feed".to_string(),
            rationale: "No stable direct-read explorer contract is carved out yet from the current RPC surface.".to_string(),
            allowed_methods: Vec::new(),
        },
    ];
    routes.sort_by(|a, b| a.route_id.cmp(&b.route_id));

    FrontendRouteAllowlist {
        generated_at: generated_at.to_string(),
        source_consumer_contracts_file: "rpc_consumer_contracts.json".to_string(),
        routes,
    }
}

fn filter_methods(entries: &[RpcConsumerContractEntry], wanted: &[&str]) -> Vec<String> {
    wanted
        .iter()
        .filter_map(|name| {
            entries
                .iter()
                .find(|entry| entry.method == *name)
                .map(|entry| entry.method.clone())
        })
        .collect()
}

fn build_sidecar_adapter_backlog(
    generated_at: &str,
    contracts: &RpcConsumerContracts,
) -> SidecarAdapterBacklog {
    let mut backlog = Vec::new();
    for entry in &contracts.sidecar_only_methods {
        let (route_id, route_label, backlog_reason) = sidecar_route_assignment(entry);
        backlog.push(SidecarAdapterBacklogEntry {
            route_id: route_id.to_string(),
            route_label: route_label.to_string(),
            method: entry.method.clone(),
            backlog_reason: backlog_reason.to_string(),
            ownership_note: entry.ownership_note.clone(),
            notes: entry.notes.clone(),
        });
    }
    backlog.sort_by(|a, b| a.route_id.cmp(&b.route_id).then(a.method.cmp(&b.method)));

    SidecarAdapterBacklog {
        generated_at: generated_at.to_string(),
        source_consumer_contracts_file: "rpc_consumer_contracts.json".to_string(),
        backlog,
    }
}

fn sidecar_route_assignment(
    entry: &RpcConsumerContractEntry,
) -> (&'static str, &'static str, &'static str) {
    match entry.method.as_str() {
        "x3_submitCrossVmTransaction" | "x3_submitSvmTransaction" => (
            "bridge-status",
            "Bridge Status",
            "Bridge submission stays behind the sidecar because queueing, billing, and orchestration remain node-owned.",
        ),
        "submitDispute" => (
            "governance",
            "Governance Desk",
            "Governance actions must stay behind a sidecar or signer boundary even when read visibility is frontend-safe.",
        ),
        "gpu_submitProof" => (
            "network-overview",
            "Network Overview",
            "Proof submission is an operator or service action, not a direct frontend read contract.",
        ),
        "x3_estimateGas" | "x3_isAuthorized" => (
            "wallet-home",
            "Wallet Home",
            "Wallet-facing action preparation still depends on adapter-owned semantics and should not bind directly to frontend routes.",
        ),
        "x3_flashFinalityStatus" => (
            "network-overview",
            "Network Overview",
            "Flash finality status should stay sidecar-owned until conditional registration semantics stay intentionally documented and tested.",
        ),
        _ if entry.method.starts_with("atomicTrade_") || entry.method.starts_with("walletDex_") => (
            "wallet-home",
            "Wallet Home",
            "Trading and swap flows remain sidecar-owned because they are backed by node-local services rather than stable direct-read contracts.",
        ),
        _ => (
            "explorer",
            "Explorer Feed",
            "No route-specific frontend-safe contract exists yet, so keep this behind sidecar ownership until the consumer surface freezes.",
        ),
    }
}

fn render_frontend_route_allowlist_markdown(allowlist: &FrontendRouteAllowlist) -> String {
    let mut out = String::new();
    out.push_str("# Generated Frontend Route Allowlist\n\n");
    out.push_str(
        "This file is generated from rpc_consumer_contracts.json. Do not hand-edit it.\n\n",
    );
    out.push_str(&format!("Generated at: {}\n\n", allowlist.generated_at));
    out.push_str("| Route | Allowed Methods | Rationale |\n");
    out.push_str("| --- | --- | --- |\n");
    for route in &allowlist.routes {
        let methods = if route.allowed_methods.is_empty() {
            "-".to_string()
        } else {
            route.allowed_methods.join(", ")
        };
        out.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            route.route_id, methods, route.rationale
        ));
    }
    out
}

fn render_sidecar_adapter_backlog_markdown(backlog: &SidecarAdapterBacklog) -> String {
    let mut out = String::new();
    out.push_str("# Generated Sidecar Adapter Backlog\n\n");
    out.push_str(
        "This file is generated from rpc_consumer_contracts.json. Do not hand-edit it.\n\n",
    );
    out.push_str(&format!("Generated at: {}\n\n", backlog.generated_at));
    out.push_str("| Route | Method | Reason | Ownership Note | Notes |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    for item in &backlog.backlog {
        let ownership_note = item.ownership_note.as_deref().unwrap_or("-");
        let notes = if item.notes.is_empty() {
            "-".to_string()
        } else {
            item.notes.join("; ")
        };
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} |\n",
            item.route_id, item.method, item.backlog_reason, ownership_note, notes
        ));
    }
    out
}

fn normalize_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn brace_delta(line: &str) -> i32 {
    let opens = line.chars().filter(|ch| *ch == '{').count() as i32;
    let closes = line.chars().filter(|ch| *ch == '}').count() as i32;
    opens - closes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_runtime_impl_methods() {
        let root = std::env::temp_dir().join(format!(
            "launchops-inventory-runtime-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("runtime/src")).expect("runtime dir");
        let runtime = root.join("runtime/src/lib.rs");
        std::fs::write(
            &runtime,
            r#"
#[cfg(feature = "gpu-validator")]
impl foo::BarApi<Block> for Runtime {
    fn first() -> u32 { 1 }
    fn second(arg: u32) -> u32 {
        arg
    }
}
"#,
        )
        .expect("write runtime");

        let traits = parse_runtime_traits(&runtime).expect("parse runtime");
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].trait_name, "foo::BarApi<Block>");
        assert_eq!(
            traits[0].cfg_guard.as_deref(),
            Some("#[cfg(feature = \"gpu-validator\")]")
        );
        assert_eq!(traits[0].methods.len(), 2);
        assert_eq!(traits[0].methods[0].name, "first");
        assert_eq!(traits[0].methods[1].name, "second");
    }

    #[test]
    fn classifies_rpc_buckets() {
        let root = std::env::temp_dir().join(format!(
            "launchops-inventory-rpc-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("node/src")).expect("rpc dir");
        let rpc = root.join("node/src/rpc.rs");
        std::fs::write(
            &rpc,
            r#"
module.register_method("x3_getAssetMetadata", move |params, _| {
    let api = c.runtime_api();
    api.get_asset_metadata(at, 1)
})?;

module.register_method("walletDex_estimateSwap", move |params, _| {
    wallet_dex.estimate_swap(request)
})?;

module.register_method("x3_submitX3vmTransaction", move |params, _| {
    Err::<String, _>(custom_error(
        "x3_newCore is not available on this node build",
    ))
})?;
"#,
        )
        .expect("write rpc");

        let mut method_to_traits = BTreeMap::new();
        method_to_traits.insert(
            "get_asset_metadata".to_string(),
            vec![
                "pallet_x3_kernel::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>"
                    .to_string(),
            ],
        );

        let methods = parse_rpc_methods(&rpc, &method_to_traits).expect("parse rpc");
        assert_eq!(methods.len(), 3);
        assert!(matches!(
            methods[0].method.bucket,
            RpcContractBucket::RuntimeBacked
        ));
        assert!(matches!(
            methods[1].method.bucket,
            RpcContractBucket::NodeLocalAdapter
        ));
        assert!(matches!(
            methods[2].method.bucket,
            RpcContractBucket::Placeholder
        ));
    }

    #[test]
    fn detects_duplicate_registrations_and_builds_consumer_split() {
        let methods = vec![
            ParsedRpcRegistration {
                branch_kind: None,
                branch_condition: None,
                method: RpcMethodContract {
                    method: "x3_getAssetMetadata".to_string(),
                    source_file: RPC_SOURCE.to_string(),
                    line: 10,
                    bucket: RpcContractBucket::RuntimeBacked,
                    runtime_calls: vec!["get_asset_metadata".to_string()],
                    runtime_trait_hints: vec!["AtlasKernelRuntimeApi".to_string()],
                    node_local_signals: vec![],
                    placeholder_reason: None,
                    ownership_note: None,
                    frontend_consumer_mode: "direct_read_candidate".to_string(),
                    sidecar_consumer_mode: "pass_through_candidate".to_string(),
                },
            },
            ParsedRpcRegistration {
                branch_kind: None,
                branch_condition: None,
                method: RpcMethodContract {
                    method: "x3_getAssetMetadata".to_string(),
                    source_file: RPC_SOURCE.to_string(),
                    line: 42,
                    bucket: RpcContractBucket::NodeLocalAdapter,
                    runtime_calls: vec!["get_asset_metadata".to_string()],
                    runtime_trait_hints: vec!["AtlasKernelRuntimeApi".to_string()],
                    node_local_signals: vec!["BillingMiddleware quota path".to_string()],
                    placeholder_reason: None,
                    ownership_note: None,
                    frontend_consumer_mode: "adapter_only".to_string(),
                    sidecar_consumer_mode: "orchestrate".to_string(),
                },
            },
            ParsedRpcRegistration {
                branch_kind: None,
                branch_condition: None,
                method: RpcMethodContract {
                    method: "requestProofChallenge".to_string(),
                    source_file: RPC_SOURCE.to_string(),
                    line: 60,
                    bucket: RpcContractBucket::Placeholder,
                    runtime_calls: vec![],
                    runtime_trait_hints: vec![],
                    node_local_signals: vec![],
                    placeholder_reason: Some("structural placeholder".to_string()),
                    ownership_note: None,
                    frontend_consumer_mode: "mock_only".to_string(),
                    sidecar_consumer_mode: "defer".to_string(),
                },
            },
        ];

        let flags = build_contract_flags(&methods);
        assert!(flags.iter().any(|flag| {
            flag.category == RpcContractFlagCategory::DuplicateRegistration
                && flag.method == "x3_getAssetMetadata"
        }));
        assert!(flags.iter().any(|flag| {
            flag.category == RpcContractFlagCategory::BucketDrift
                && flag.method == "x3_getAssetMetadata"
        }));

        let contracts = build_consumer_contracts("2026-04-22T00:00:00Z", &methods);
        assert_eq!(contracts.frontend_safe_count, 0);
        assert_eq!(contracts.sidecar_only_count, 1);
        assert_eq!(contracts.mock_only_count, 1);
        assert_eq!(
            contracts.sidecar_only_methods[0].method,
            "x3_getAssetMetadata"
        );
        assert_eq!(contracts.sidecar_only_methods[0].registration_count, 2);
    }

    #[test]
    fn suppresses_expected_conditional_duplicate_flags() {
        let methods = vec![
            ParsedRpcRegistration {
                branch_kind: Some(BranchKind::If),
                branch_condition: Some("if let Some(gadget) = flash_finality_gadget".to_string()),
                method: RpcMethodContract {
                    method: "x3_flashFinalityStatus".to_string(),
                    source_file: RPC_SOURCE.to_string(),
                    line: 10,
                    bucket: RpcContractBucket::NodeLocalAdapter,
                    runtime_calls: vec![],
                    runtime_trait_hints: vec![],
                    node_local_signals: vec!["flash finality gadget".to_string()],
                    placeholder_reason: None,
                    ownership_note: None,
                    frontend_consumer_mode: "adapter_only".to_string(),
                    sidecar_consumer_mode: "orchestrate".to_string(),
                },
            },
            ParsedRpcRegistration {
                branch_kind: Some(BranchKind::Else),
                branch_condition: Some("if let Some(gadget) = flash_finality_gadget".to_string()),
                method: RpcMethodContract {
                    method: "x3_flashFinalityStatus".to_string(),
                    source_file: RPC_SOURCE.to_string(),
                    line: 20,
                    bucket: RpcContractBucket::NodeLocalAdapter,
                    runtime_calls: vec![],
                    runtime_trait_hints: vec![],
                    node_local_signals: vec![],
                    placeholder_reason: None,
                    ownership_note: None,
                    frontend_consumer_mode: "adapter_only".to_string(),
                    sidecar_consumer_mode: "orchestrate".to_string(),
                },
            },
        ];

        let flags = build_contract_flags(&methods);
        assert!(!flags
            .iter()
            .any(|flag| flag.category == RpcContractFlagCategory::DuplicateRegistration));
    }

    #[test]
    fn marks_expected_mixed_ownership_without_drift_flag() {
        let runtime_calls = BTreeSet::from(["submit_evm_transaction".to_string()]);
        let node_local_signals = vec![
            "BillingMiddleware quota path".to_string(),
            "CrossVmBridge node-local queue".to_string(),
        ];
        let methods = vec![ParsedRpcRegistration {
            branch_kind: None,
            branch_condition: None,
            method: RpcMethodContract {
                method: "x3_submitCrossVmTransaction".to_string(),
                source_file: RPC_SOURCE.to_string(),
                line: 30,
                bucket: RpcContractBucket::NodeLocalAdapter,
                runtime_calls: runtime_calls.iter().cloned().collect(),
                runtime_trait_hints: vec!["AtlasKernelRuntimeApi".to_string()],
                node_local_signals: node_local_signals.clone(),
                placeholder_reason: None,
                ownership_note: detect_expected_mixed_ownership(
                    "x3_submitCrossVmTransaction",
                    &runtime_calls,
                    &node_local_signals,
                ),
                frontend_consumer_mode: "adapter_only".to_string(),
                sidecar_consumer_mode: "orchestrate".to_string(),
            },
        }];

        let flags = build_contract_flags(&methods);
        assert!(!flags
            .iter()
            .any(|flag| flag.category == RpcContractFlagCategory::BucketDrift));
    }
}
