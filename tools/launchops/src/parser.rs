use regex::Regex;

use crate::classifier::{classify_module, classify_risk};
use crate::models::{Requirement, RequirementStatus};

pub fn parse_requirements_from_markdown(path: &str, content: &str) -> Vec<Requirement> {
    let marker_re =
        Regex::new(r"^\s*[-*]\s*\[(?P<mark>[xX ~!#])\]\s*(?P<text>.+)$").expect("valid regex");
    let tag_re =
        Regex::new(r"\[(?P<tag>MAINNET|TESTNET|SECURITY|CONSENSUS|BRIDGE|CROSS_VM|DEX|GPU|OPS)\]")
            .expect("valid regex");

    let mut out = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        if let Some(caps) = marker_re.captures(line) {
            let mark = caps.name("mark").map(|m| m.as_str()).unwrap_or(" ");
            let raw_text = caps.name("text").map(|m| m.as_str()).unwrap_or("").trim();

            let tags: Vec<String> = tag_re
                .captures_iter(raw_text)
                .filter_map(|m| m.name("tag").map(|t| t.as_str().to_string()))
                .collect();

            let cleaned = tag_re.replace_all(raw_text, "").trim().to_string();

            let status = match mark {
                "x" | "X" => RequirementStatus::Complete,
                "~" => RequirementStatus::Partial,
                "!" => RequirementStatus::Blocker,
                "#" => RequirementStatus::NeedsTest,
                _ => RequirementStatus::Incomplete,
            };

            let mut req = Requirement {
                id: slugify(&cleaned),
                text: cleaned,
                source_file: path.to_string(),
                line: idx + 1,
                status,
                tags,
                module: crate::models::Module::Unknown,
                risk: crate::models::RiskLevel::Low,
            };
            req.module = classify_module(&req.text, &req.tags);
            req.risk = classify_risk(&req);
            out.push(req);
        }
    }

    out
}

pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in text.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markers_and_tags() {
        let content = "- [x] [CROSS_VM] Cross-VM atomic swap implemented\n- [!] [BRIDGE][SECURITY] Replay protection missing\n- [#] Needs fuzz test";
        let reqs = parse_requirements_from_markdown("docs/test.md", content);
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].status, RequirementStatus::Complete);
        assert_eq!(reqs[1].status, RequirementStatus::Blocker);
        assert_eq!(reqs[2].status, RequirementStatus::NeedsTest);
        assert!(reqs[0].tags.contains(&"CROSS_VM".to_string()));
    }
}
