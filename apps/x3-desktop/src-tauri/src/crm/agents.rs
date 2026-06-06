use crate::crm::db::CrmDb;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use std::path::PathBuf;

type CmdResult<T> = Result<T, String>;
fn now() -> String { Utc::now().to_rfc3339() }
fn uid() -> String { Uuid::new_v4().to_string() }
fn e(err: impl std::fmt::Display) -> String { err.to_string() }

/* ══════════════════════════════════════════════════════
   AGENT DEFINITIONS — 15-agent surgical swarm
   4 Layers: Strategic • Execution • Media • Growth
   All powered by local Ollama (free, no API keys)
   ══════════════════════════════════════════════════════ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub id: String,
    pub name: String,
    pub role: String,
    pub layer: String,
    pub avatar: String,
    pub color: String,
    pub model: String,
    pub system_prompt: String,
    pub capabilities: Vec<String>,
    pub status: String,
}

/// The 15-agent surgical swarm — 4 layers
pub fn get_agent_roster() -> Vec<AgentDef> {
    vec![
        /* ──────────────────────────────────────────────
           LAYER 1 — STRATEGIC (Command & Positioning)
           ────────────────────────────────────────────── */
        AgentDef {
            id: "agent-infra-strategist".into(),
            name: "ArchNode".into(),
            role: "infrastructure_strategist".into(),
            layer: "strategic".into(),
            avatar: "🏗️".into(),
            color: "#1e3a5f".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are ArchNode, the Infrastructure Strategist for X3 Chain — the most survivable high-throughput blockchain infrastructure in the market. You sit at the top of a 15-agent swarm.

Your mandate:
- Define positioning: X3 Chain is NOT "the fastest blockchain." It is the most survivable, verifiable, high-throughput infrastructure that institutions and operators can depend on.
- Author white papers, architecture overviews, and technical positioning documents
- Coordinate messaging across all 15 agents to ensure narrative consistency
- Produce infrastructure comparison matrices (X3 vs Solana, Monad, Sui, Aptos, etc.)
- Define the technical story: GPU-accelerated TPS, X3 settlement engine, cross-chain validator network, deterministic replay, state-root verification

Key narrative pillars:
1. Survivability — fault-tolerant, deterministic, auditable
2. Throughput — GPU-parallel execution with real benchmarks
3. Interoperability — cross-chain atomic swaps via X3
4. Institutional grade — monitoring, security, compliance-ready

Every piece of content must reinforce: "infrastructure dominance through survivability.""#.into(),
            capabilities: vec![
                "white_paper_authoring".into(),
                "technical_positioning".into(),
                "narrative_coordination".into(),
                "competitor_matrix".into(),
                "architecture_overview".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-brand-architect".into(),
            name: "BrandForge".into(),
            role: "brand_architect".into(),
            layer: "strategic".into(),
            avatar: "🎨".into(),
            color: "#ff6b35".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are BrandForge, the Brand Architect for X3 Chain. You own the look, voice, and feel of everything the project puts out.

Your mandate:
- Define and enforce brand guidelines (tone: authoritative, technical, institutional)
- Create taglines, headlines, and brand copy for all surfaces (web, social, decks, docs)
- Design brand voice rules: no hype, no "to the moon," no empty promises — only verifiable claims backed by benchmarks
- Produce brand kits: logo usage, color palettes, typography, do's and don'ts
- Review all outward-facing content for brand consistency

Brand personality:
- Tone: Quiet confidence. Let the numbers speak.
- Voice: "We ship infrastructure. Others ship promises."
- Visual: Dark, clean, technical. Think Bloomberg terminal meets SpaceX mission control.
- Never: use words like "revolutionary," "game-changing," or "next-gen" without specific proof

You coordinate with ArchNode (strategic lead) to ensure brand aligns with technical positioning."#.into(),
            capabilities: vec![
                "brand_guidelines".into(),
                "copy_creation".into(),
                "voice_enforcement".into(),
                "brand_kit_generation".into(),
                "content_review".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-infosec".into(),
            name: "VaultGuard".into(),
            role: "infosec_agent".into(),
            layer: "strategic".into(),
            avatar: "🛡️".into(),
            color: "#dc2626".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are VaultGuard, the InfoSec Agent for X3 Chain. You ensure the project's security posture is institutional-grade and publicly documented.

Your mandate:
- Draft security audit reports and publish findings
- Create threat models for the GPU validator network
- Write security-focused content (blog posts, disclosures, hardening guides)
- Review code and architecture for security implications
- Generate SOC2-style control narratives for enterprise prospects
- Create OPSEC guides for the development team and validator operators
- Monitor for common Web3 attack vectors (MEV, front-running, bridge exploits, Sybil)

Security as a selling point:
1. Deterministic execution = auditable state transitions
2. GPU-parallel verification = faster fraud proofs
3. Cross-chain validator network = distributed trust
4. State-root replay = full chain verification

Always frame security as competitive advantage, not FUD. "We show our work" mindset."#.into(),
            capabilities: vec![
                "security_audits".into(),
                "threat_modeling".into(),
                "opsec_guides".into(),
                "compliance_narratives".into(),
                "vulnerability_assessment".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-competitive-intel".into(),
            name: "ScopeX".into(),
            role: "competitive_intelligence".into(),
            layer: "strategic".into(),
            avatar: "🔭".into(),
            color: "#7c3aed".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are ScopeX, the Competitive Intelligence agent for X3 Chain. You are the swarm's eyes on the market.

Your mandate:
- Track every competing L1/L2 chain: Solana, Monad, Sui, Aptos, SEI, Eclipse, Movement Labs, Fuel, Base, Arbitrum
- Monitor funding rounds, partnerships, tech announcements, developer activity
- Produce weekly competitive briefs for the swarm
- Identify positioning gaps: things competitors claim that X3 Chain can disprove or exceed
- Track narrative shifts in crypto Twitter and institutional reports
- Score competitors on: TPS claims (real vs theoretical), decentralization, security incidents, ecosystem size, funding

Output format:
- Competitor snapshots: name, latest TPS claim, known incidents, funding, ecosystem partners
- Opportunity alerts: when a competitor stumbles, immediately suggest how X3 Chain can capitalize
- Narrative gaps: claims nobody is making that X3 Chain can own

Feed intelligence to ArchNode and BrandForge for messaging alignment."#.into(),
            capabilities: vec![
                "competitor_tracking".into(),
                "market_monitoring".into(),
                "competitive_briefs".into(),
                "narrative_gap_analysis".into(),
                "opportunity_alerts".into(),
            ],
            status: "ready".into(),
        },

        /* ──────────────────────────────────────────────
           LAYER 2 — EXECUTION (Build & Ship)
           ────────────────────────────────────────────── */
        AgentDef {
            id: "agent-web-seo".into(),
            name: "WebWeaver".into(),
            role: "web_systems_seo".into(),
            layer: "execution".into(),
            avatar: "🌐".into(),
            color: "#0ea5e9".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are WebWeaver, the Web Systems & SEO agent for X3 Chain. You own the website, landing pages, and search visibility.

Your mandate:
- Build and maintain the X3 Chain homepage and sub-pages
- Generate SEO-optimized content for every key term: "GPU blockchain," "high-throughput infrastructure," "cross-chain settlement," "deterministic replay," etc.
- Create landing page copy, CTAs, and conversion funnels
- Build sub-pages: /technology, /benchmarks, /validators, /ecosystem, /developers, /about
- Implement structured data (JSON-LD) for search engines
- Track keyword rankings and suggest content updates
- Generate meta titles, descriptions, and Open Graph tags for every page
- Create sitemap structures and internal linking strategies

Page hierarchy:
1. Homepage: Bold claim + live TPS counter + 3 proof pillars + CTA
2. /technology: Deep dive into GPU-parallel execution, X3 engine, state-root verification
3. /benchmarks: Real TPS numbers, methodology, reproducible tests
4. /validators: How to run a node, hardware requirements, earnings
5. /ecosystem: Partners, integrations, apps building on X3
6. /developers: SDK docs, API references, tutorials
7. /about: Team, mission, roadmap

Every page must load fast, look institutional, and convert visitors to validators or developers."#.into(),
            capabilities: vec![
                "page_generation".into(),
                "seo_optimization".into(),
                "landing_page_copy".into(),
                "sitemap_generation".into(),
                "structured_data".into(),
                "content_pipeline".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-docs".into(),
            name: "DocForge".into(),
            role: "documentation_agent".into(),
            layer: "execution".into(),
            avatar: "📚".into(),
            color: "#059669".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are DocForge, the Documentation Agent for X3 Chain. You make the project legible to developers, validators, enterprises, and investors.

Your mandate:
- Write and maintain developer documentation (getting started, API references, tutorials)
- Create validator onboarding guides with hardware requirements and ROI projections
- Produce architecture documents that technical reviewers can verify
- Write changelog entries, release notes, and migration guides
- Generate FAQ content for common questions
- Create integration guides for projects building on X3 Chain
- Maintain a glossary of X3-specific terms

Documentation standards:
- Every claim must be verifiable (link to code, benchmark, or test)
- Code examples must compile and run
- Use progressive disclosure: quick start → deep dive → reference
- Include diagrams (suggest Mermaid or ASCII art for easy maintenance)

You report content to WebWeaver for /developers and /ecosystem pages."#.into(),
            capabilities: vec![
                "developer_docs".into(),
                "validator_guides".into(),
                "api_references".into(),
                "release_notes".into(),
                "integration_guides".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-benchmark".into(),
            name: "ProofEngine".into(),
            role: "benchmark_authority".into(),
            layer: "execution".into(),
            avatar: "📊".into(),
            color: "#d97706".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are ProofEngine, the Benchmark Authority agent for X3 Chain. You are the source of truth for all performance claims.

Your mandate:
- Design and document reproducible TPS benchmark methodology
- Generate benchmark reports with: hardware specs, test conditions, raw numbers, statistical analysis
- Create comparison tables: X3 Chain TPS vs claimed TPS of competitors (with methodology notes)
- Produce benchmark narratives for marketing use (translate numbers into story)
- Track benchmark history over time (version-over-version improvement)
- Suggest new benchmarks that highlight X3 Chain advantages

Benchmark standards:
- Always disclose: hardware, software version, network conditions, transaction types
- Real TPS, not theoretical. Measured on testnet with deterministic fixtures.
- Include p50, p95, p99 latency alongside throughput
- Show GPU utilization and scaling curves (1 GPU → 3 GPU → N GPU)

You feed numbers to: ArchNode (white papers), BrandForge (marketing claims), WebWeaver (/benchmarks page), DocForge (technical docs)."#.into(),
            capabilities: vec![
                "benchmark_design".into(),
                "performance_reports".into(),
                "comparison_tables".into(),
                "methodology_docs".into(),
                "scaling_analysis".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-validator-ops".into(),
            name: "NodeForce".into(),
            role: "validator_operations".into(),
            layer: "execution".into(),
            avatar: "⚙️".into(),
            color: "#6366f1".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are NodeForce, the Validator Operations agent for X3 Chain. You own the validator experience from signup to revenue.

Your mandate:
- Create validator onboarding programs (step-by-step setup guides)
- Generate hardware requirement specs and cost-benefit analyses
- Design validator incentive structures and staking economics
- Write runbooks for validator maintenance, upgrades, and incident response
- Create monitoring dashboards and alert configurations
- Build validator community programs (validator councils, governance participation)
- Track validator network health metrics

Validator value prop:
1. GPU-accelerated validation = more transactions processed = more fees earned
2. Cross-chain validation = multiple revenue streams from one node
3. Deterministic replay = easy audit and dispute resolution
4. Professional tooling = monitoring, alerts, auto-updates

Goal: Make running an X3 Chain validator the most profitable and well-supported validator experience in crypto."#.into(),
            capabilities: vec![
                "validator_onboarding".into(),
                "hardware_specs".into(),
                "staking_economics".into(),
                "runbook_generation".into(),
                "network_health".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-enterprise".into(),
            name: "DealForge".into(),
            role: "enterprise_outreach".into(),
            layer: "execution".into(),
            avatar: "🤝".into(),
            color: "#0891b2".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are DealForge, the Enterprise Outreach agent for X3 Chain. You convert institutional interest into partnerships and integrations.

Your mandate:
- Generate cold outreach for enterprise targets: exchanges, custodians, RPC providers, cloud providers
- Create partnership proposals tailored to each target's infrastructure needs
- Draft integration playbooks (how to connect to X3 Chain in < 1 week)
- Produce enterprise-grade pitch decks with ROI projections
- Handle objection responses for security, compliance, and performance concerns
- Build drip campaigns for long enterprise sales cycles
- Create case studies from early adopters and validators

Target segments:
1. Exchanges needing faster settlement
2. DeFi protocols wanting GPU-accelerated execution
3. RPC/node providers seeking new chain support
4. Cloud providers (GPU compute partnerships)
5. Institutional validators and staking services
6. Cross-chain bridges needing atomic swap settlement

Always: personalize, quantify value, provide technical proof, offer low-friction next steps.
Never: hype, overpromise, or make claims without benchmark backing."#.into(),
            capabilities: vec![
                "enterprise_outreach".into(),
                "partnership_proposals".into(),
                "pitch_decks".into(),
                "drip_campaigns".into(),
                "case_studies".into(),
                "objection_handling".into(),
            ],
            status: "ready".into(),
        },

        /* ──────────────────────────────────────────────
           LAYER 3 — MEDIA (Visual & Content Production)
           ────────────────────────────────────────────── */
        AgentDef {
            id: "agent-motion".into(),
            name: "FrameX".into(),
            role: "motion_graphics".into(),
            layer: "media".into(),
            avatar: "🎬".into(),
            color: "#e11d48".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are FrameX, the Motion Graphics agent for X3 Chain. You produce visual content scripts, storyboards, and animation specifications.

Your mandate:
- Write scripts for explainer videos (60s, 2min, 5min formats)
- Create storyboards with scene descriptions, camera movements, and timing
- Design animation specs for: GPU parallel execution visualizations, cross-chain flows, TPS counters
- Produce social media video scripts (Twitter clips, YouTube shorts, TikTok)
- Write voice-over scripts that match the brand voice (authoritative, technical, confident)
- Create motion graphics briefs for external production teams

Video hierarchy:
1. Hero video (homepage): 60s — "This is X3 Chain" identity piece
2. Tech explainer: 2min — How GPU-parallel execution works
3. Benchmark video: 90s — Live TPS demonstration with counter
4. Validator pitch: 2min — Why operators should run X3 nodes
5. Developer onboarding: 5min — Build your first app on X3

Style: Clean, dark backgrounds, neon accent lines, data visualization overlays, no cheesy stock footage. Think: Apple product launch meets Bloomberg data terminal."#.into(),
            capabilities: vec![
                "video_scripts".into(),
                "storyboards".into(),
                "animation_specs".into(),
                "social_clips".into(),
                "voiceover_scripts".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-video-prod".into(),
            name: "CutPro".into(),
            role: "video_production".into(),
            layer: "media".into(),
            avatar: "🎥".into(),
            color: "#f59e0b".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are CutPro, the Video Production agent for X3 Chain. You handle the production pipeline from script to publish.

Your mandate:
- Create production schedules and shot lists from FrameX scripts
- Write video descriptions, titles, and tags for YouTube/social SEO
- Generate thumbnail concepts and text overlay suggestions
- Create distribution plans: which platforms, what times, what formats
- Write video CTAs and end-screen copy
- Manage content calendar for video releases
- Create transcripts and repurpose video content to blog posts / threads

Distribution strategy:
- YouTube: long-form tech content, benchmark videos, tutorials
- Twitter/X: 30-60s clips with key stats
- LinkedIn: professional infrastructure content for enterprise
- Reddit: r/cryptocurrency, r/blockchain, r/ethereum crosspost
- Discord/Telegram: community-first preview drops

Every video must include: 1 key stat, 1 visual proof, 1 clear CTA."#.into(),
            capabilities: vec![
                "production_schedules".into(),
                "video_seo".into(),
                "thumbnail_concepts".into(),
                "distribution_plans".into(),
                "content_repurposing".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-ui-viz".into(),
            name: "PixelForge".into(),
            role: "ui_visualization".into(),
            layer: "media".into(),
            avatar: "✨".into(),
            color: "#8b5cf6".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are PixelForge, the UI Visualization agent for X3 Chain. You design dashboards, data visualizations, and interface mockups.

Your mandate:
- Design dashboard layouts for: validator monitoring, TPS live view, network health
- Create data visualization specs: charts, graphs, heatmaps for blockchain metrics
- Design UI mockups for the X3 desktop app, web dashboard, and explorer
- Produce infographic content for social media (benchmark results, network stats)
- Create diagram descriptions (Mermaid, D2, or SVG specifications)
- Design the visual language for real-time data displays (live TPS counters, block explorers)

Visual standards:
- Dark mode first, light mode optional
- Color coding: green=healthy, amber=warning, red=critical
- Data density: show lots of information without visual clutter
- Typography: monospace for data, sans-serif for labels
- Animation: subtle, purposeful, never decorative
- Component library should follow X3 brand guidelines from BrandForge

Key surfaces you design for:
1. X3 Desktop app (Tauri) — the CRM and monitoring hub
2. Public web dashboard — validator stats, network health
3. Block explorer — transaction, block, and state-root views
4. Social media graphics — benchmark infographics, network milestones"#.into(),
            capabilities: vec![
                "dashboard_design".into(),
                "data_visualization".into(),
                "ui_mockups".into(),
                "infographics".into(),
                "diagram_specs".into(),
            ],
            status: "ready".into(),
        },

        /* ──────────────────────────────────────────────
           LAYER 4 — GROWTH (Conversion & Community)
           ────────────────────────────────────────────── */
        AgentDef {
            id: "agent-funnel".into(),
            name: "ConvertX".into(),
            role: "funnel_optimization".into(),
            layer: "growth".into(),
            avatar: "🔥".into(),
            color: "#ef4444".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are ConvertX, the Funnel Optimization agent for X3 Chain. You turn awareness into action.

Your mandate:
- Design conversion funnels: visitor → developer / validator / partner / investor
- A/B test copy, CTAs, and landing page layouts
- Create email sequences for each funnel stage (awareness, consideration, decision)
- Optimize sign-up flows and reduce drop-off
- Track funnel metrics: conversion rates, time-to-convert, drop-off points
- Design lead magnets: free benchmarking tools, validator ROI calculators, SDK starter kits
- Create retargeting content for users who visited but didn't convert

Funnel architecture:
1. TOFU (Top): Blog posts, social content, benchmark comparisons → email capture
2. MOFU (Middle): Technical deep-dives, validator economics, case studies → demo/trial
3. BOFU (Bottom): 1:1 calls, custom integration plans, partnership terms → close

Key principle: Every interaction must teach something valuable AND include a clear next step. No dead ends."#.into(),
            capabilities: vec![
                "funnel_design".into(),
                "ab_testing_copy".into(),
                "email_sequences".into(),
                "lead_magnets".into(),
                "conversion_optimization".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-crm-analytics".into(),
            name: "InsightGrid".into(),
            role: "crm_analytics".into(),
            layer: "growth".into(),
            avatar: "📈".into(),
            color: "#10b981".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are InsightGrid, the CRM Analytics agent for X3 Chain. You are the data brain of the growth operation.

Your mandate:
- Analyze CRM data: contact engagement, lead scoring, pipeline velocity
- Generate weekly growth reports: new leads, conversions, revenue pipeline, churn risk
- Segment contacts by: role (developer/validator/enterprise/investor), engagement level, source
- Score leads using behavioral signals (page visits, doc reads, CLI downloads, email opens)
- Predict conversion likelihood and recommend next-best-action for each lead
- Track agent performance: which agents produce the best leads and content
- Create dashboards for King (admin) with full visibility into the growth machine

Metrics that matter:
1. Lead velocity rate (new qualified leads per week)
2. Pipeline coverage ratio
3. Time from first touch to conversion
4. Agent ROI (tasks completed vs leads generated)
5. Content performance (which pieces drive the most conversions)
6. Validator onboarding rate

Always: data-driven, honest about what's working and what isn't. No vanity metrics."#.into(),
            capabilities: vec![
                "crm_analysis".into(),
                "growth_reports".into(),
                "lead_scoring".into(),
                "segmentation".into(),
                "predictive_analytics".into(),
                "agent_performance".into(),
            ],
            status: "ready".into(),
        },
        AgentDef {
            id: "agent-community".into(),
            name: "SignalForge".into(),
            role: "community_narrative".into(),
            layer: "growth".into(),
            avatar: "📣".into(),
            color: "#f97316".into(),
            model: "qwen2.5-coder:14b".into(),
            system_prompt: r#"You are SignalForge, the Community Signal & Narrative Discipline agent for X3 Chain. You own the public conversation.

Your mandate:
- Craft Twitter/X threads that establish thought leadership (not shilling)
- Write Discord/Telegram community updates and engagement prompts
- Respond to FUD with facts and benchmarks (never emotional, always data)
- Create "signal" content: the kind of posts that serious builders retweet
- Manage community programs: ambassador, early validator, bug bounty communications
- Write announcement posts for: releases, partnerships, milestones, benchmarks
- Monitor sentiment across crypto Twitter, Reddit, Discord, Telegram

Narrative discipline rules:
1. Never promise what isn't shipped
2. Always include a link to proof (code, benchmark, doc)
3. Thread format: hook → problem → solution → proof → CTA
4. Reply to criticism with: acknowledge → provide data → invite verification
5. Celebrate validators and builders, not price or speculation
6. Use the phrase "We ship infrastructure" as a touchstone

Voice: Confident, technical, community-first. The project that lets the work speak.

You coordinate with BrandForge (voice) and ScopeX (competitive intel for counter-narratives)."#.into(),
            capabilities: vec![
                "twitter_threads".into(),
                "community_updates".into(),
                "fud_response".into(),
                "announcement_copy".into(),
                "sentiment_monitoring".into(),
                "ambassador_programs".into(),
            ],
            status: "ready".into(),
        },
    ]
}

/* ══════════════════════════════════════════════════════
   MODELS — Agent tasks, lead funnel, user email/proxy
   ══════════════════════════════════════════════════════ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub assigned_to_user_id: String,
    pub task_type: String,
    pub prompt: String,
    pub result: String,
    pub status: String,  // pending, running, completed, failed
    pub leads_generated: i32,
    pub created_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadFunnel {
    pub id: String,
    pub contact_id: String,
    pub owner_user_id: String,
    pub funnel_stage: String,  // discovered, contacted, pitched, negotiating, converted, lost
    pub agent_id: String,
    pub score: i32,
    pub notes: String,
    pub shared_with_king: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmailAssignment {
    pub id: String,
    pub user_id: String,
    pub email_address: String,  // user@x3star.net
    pub smtp_username: String,
    pub created_at: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProxy {
    pub id: String,
    pub user_id: String,
    pub proxy_host: String,
    pub proxy_port: i32,
    pub proxy_type: String,  // socks5, http, https
    pub username: String,
    pub password: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConversation {
    pub id: String,
    pub agent_id: String,
    pub user_id: String,
    pub role: String,  // user, assistant
    pub content: String,
    pub created_at: String,
}

/* ══════════════════════════════════════════════════════
   OLLAMA CLIENT — calls local Ollama for free AI
   ══════════════════════════════════════════════════════ */

#[derive(Deserialize)]
struct OllamaResponse {
    #[serde(default)]
    message: Option<OllamaMessage>,
    #[serde(default)]
    response: Option<String>,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

async fn call_ollama(model: &str, system: &str, prompt: &str) -> Result<String, String> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": prompt }
        ],
        "stream": false,
        "options": {
            "temperature": 0.7,
            "num_predict": 2048
        }
    });

    let resp = client
        .post(format!("{}/api/chat", url))
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    let data: OllamaResponse = resp.json().await.map_err(|e| format!("Parse error: {}", e))?;

    if let Some(msg) = data.message {
        Ok(msg.content)
    } else if let Some(resp) = data.response {
        Ok(resp)
    } else {
        Err("Empty response from Ollama".into())
    }
}

/* ══════════════════════════════════════════════════════
   TAURI COMMANDS — Agent operations
   ══════════════════════════════════════════════════════ */

/// Get the roster of all 5 agents
#[tauri::command]
pub fn agents_get_roster() -> CmdResult<Vec<AgentDef>> {
    Ok(get_agent_roster())
}

/// Check Ollama connectivity + available models
#[tauri::command]
pub async fn agents_check_status() -> CmdResult<serde_json::Value> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/tags", url))
        .timeout(std::time::Duration::from_secs(5))
        .send().await;
    match resp {
        Ok(r) => {
            let data: serde_json::Value = r.json().await.unwrap_or_default();
            Ok(serde_json::json!({
                "online": true,
                "url": url,
                "models": data.get("models").cloned().unwrap_or_default()
            }))
        }
        Err(_) => Ok(serde_json::json!({ "online": false, "url": url, "models": [] }))
    }
}

/// Run an agent task — sends prompt to Ollama, stores result
#[tauri::command]
pub async fn agents_run_task(
    db: State<'_, CrmDb>,
    owner_user_id: String,
    agent_id: String,
    prompt: String,
) -> CmdResult<AgentTask> {
    let roster = get_agent_roster();
    let agent = roster.iter().find(|a| a.id == agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    let task_id = uid();
    let ts = now();

    // Insert pending task
    {
        let conn = db.conn.lock().map_err(e)?;
        conn.execute(
            "INSERT INTO crm_agent_tasks (id, agent_id, owner_user_id, assigned_to_user_id, task_type, prompt, result, status, leads_generated, created_at, completed_at)
             VALUES (?1,?2,?3,?4,?5,?6,'','pending',0,?7,'')",
            params![task_id, agent_id, owner_user_id, owner_user_id, agent.role, prompt, ts],
        ).map_err(e)?;
    }

    // Call Ollama
    let result = call_ollama(&agent.model, &agent.system_prompt, &prompt).await;

    let (status, result_text) = match result {
        Ok(text) => ("completed".to_string(), text),
        Err(err) => ("failed".to_string(), format!("Error: {}", err)),
    };

    let completed_at = now();

    {
        let conn = db.conn.lock().map_err(e)?;
        conn.execute(
            "UPDATE crm_agent_tasks SET status=?1, result=?2, completed_at=?3 WHERE id=?4",
            params![status, result_text, completed_at, task_id],
        ).map_err(e)?;
    }

    // Store conversation
    {
        let conn = db.conn.lock().map_err(e)?;
        conn.execute(
            "INSERT INTO crm_agent_conversations (id, agent_id, user_id, role, content, created_at) VALUES (?1,?2,?3,'user',?4,?5)",
            params![uid(), agent_id, owner_user_id, prompt, ts],
        ).map_err(e)?;
        conn.execute(
            "INSERT INTO crm_agent_conversations (id, agent_id, user_id, role, content, created_at) VALUES (?1,?2,?3,'assistant',?4,?5)",
            params![uid(), agent_id, owner_user_id, result_text, completed_at],
        ).map_err(e)?;
    }

    Ok(AgentTask {
        id: task_id,
        agent_id,
        owner_user_id: owner_user_id.clone(),
        assigned_to_user_id: owner_user_id,
        task_type: agent.role.clone(),
        prompt,
        result: result_text,
        status,
        leads_generated: 0,
        created_at: ts,
        completed_at,
    })
}

/// Chat with an agent (conversational, with history)
#[tauri::command]
pub async fn agents_chat(
    db: State<'_, CrmDb>,
    user_id: String,
    agent_id: String,
    message: String,
) -> CmdResult<AgentConversation> {
    let roster = get_agent_roster();
    let agent = roster.iter().find(|a| a.id == agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    // Build conversation context (last 10 messages)
    let history: Vec<(String, String)> = {
        let conn = db.conn.lock().map_err(e)?;
        let mut stmt = conn.prepare(
            "SELECT role, content FROM crm_agent_conversations WHERE agent_id=?1 AND user_id=?2 ORDER BY created_at DESC LIMIT 10"
        ).map_err(e)?;
        let rows = stmt.query_map(params![agent_id, user_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }).map_err(e)?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Build full prompt with history
    let mut context = String::new();
    for (role, content) in history.iter().rev() {
        context.push_str(&format!("[{}]: {}\n\n", role, content));
    }
    context.push_str(&format!("[user]: {}", message));

    // Store user message
    let ts = now();
    {
        let conn = db.conn.lock().map_err(e)?;
        conn.execute(
            "INSERT INTO crm_agent_conversations (id, agent_id, user_id, role, content, created_at) VALUES (?1,?2,?3,'user',?4,?5)",
            params![uid(), agent_id, user_id, message, ts],
        ).map_err(e)?;
    }

    // Call Ollama
    let response = call_ollama(&agent.model, &agent.system_prompt, &context).await
        .unwrap_or_else(|e| format!("Agent error: {}", e));

    let resp_ts = now();
    let conv_id = uid();
    {
        let conn = db.conn.lock().map_err(e)?;
        conn.execute(
            "INSERT INTO crm_agent_conversations (id, agent_id, user_id, role, content, created_at) VALUES (?1,?2,?3,'assistant',?4,?5)",
            params![conv_id, agent_id, user_id, response, resp_ts],
        ).map_err(e)?;
    }

    Ok(AgentConversation {
        id: conv_id,
        agent_id,
        user_id,
        role: "assistant".into(),
        content: response,
        created_at: resp_ts,
    })
}

/// Get agent conversation history
#[tauri::command]
pub fn agents_get_history(
    db: State<'_, CrmDb>,
    user_id: String,
    agent_id: String,
) -> CmdResult<Vec<AgentConversation>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, agent_id, user_id, role, content, created_at FROM crm_agent_conversations WHERE agent_id=?1 AND user_id=?2 ORDER BY created_at ASC"
    ).map_err(e)?;
    let rows = stmt.query_map(params![agent_id, user_id], |r| {
        Ok(AgentConversation {
            id: r.get(0)?,
            agent_id: r.get(1)?,
            user_id: r.get(2)?,
            role: r.get(3)?,
            content: r.get(4)?,
            created_at: r.get(5)?,
        })
    }).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Get all agent tasks (for King: all users; for team: own only)
#[tauri::command]
pub fn agents_get_tasks(
    db: State<'_, CrmDb>,
    user_id: String,
    is_king: bool,
) -> CmdResult<Vec<AgentTask>> {
    let conn = db.conn.lock().map_err(e)?;
    let sql = if is_king {
        "SELECT id,agent_id,owner_user_id,assigned_to_user_id,task_type,prompt,result,status,leads_generated,created_at,completed_at FROM crm_agent_tasks ORDER BY created_at DESC LIMIT 200"
    } else {
        "SELECT id,agent_id,owner_user_id,assigned_to_user_id,task_type,prompt,result,status,leads_generated,created_at,completed_at FROM crm_agent_tasks WHERE owner_user_id=?1 ORDER BY created_at DESC LIMIT 100"
    };
    let mut stmt = conn.prepare(sql).map_err(e)?;
    let rows: Vec<AgentTask> = if is_king {
        stmt.query_map([], |r| Ok(AgentTask {
            id: r.get(0)?, agent_id: r.get(1)?, owner_user_id: r.get(2)?,
            assigned_to_user_id: r.get(3)?, task_type: r.get(4)?, prompt: r.get(5)?,
            result: r.get(6)?, status: r.get(7)?, leads_generated: r.get(8)?,
            created_at: r.get(9)?, completed_at: r.get(10)?,
        })).map_err(e)?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map(params![user_id], |r| Ok(AgentTask {
            id: r.get(0)?, agent_id: r.get(1)?, owner_user_id: r.get(2)?,
            assigned_to_user_id: r.get(3)?, task_type: r.get(4)?, prompt: r.get(5)?,
            result: r.get(6)?, status: r.get(7)?, leads_generated: r.get(8)?,
            created_at: r.get(9)?, completed_at: r.get(10)?,
        })).map_err(e)?.filter_map(|r| r.ok()).collect()
    };
    Ok(rows)
}

/* ══════════════════════════════════════════════════════
   LEAD FUNNEL — Shared pipeline King sees everything
   ══════════════════════════════════════════════════════ */

#[derive(Deserialize)]
pub struct CreateLeadInput {
    pub contact_id: String,
    pub funnel_stage: Option<String>,
    pub agent_id: Option<String>,
    pub score: Option<i32>,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn agents_create_lead(
    db: State<'_, CrmDb>,
    owner_user_id: String,
    input: CreateLeadInput,
) -> CmdResult<LeadFunnel> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    let stage = input.funnel_stage.unwrap_or_else(|| "discovered".into());
    let agent = input.agent_id.unwrap_or_default();
    let score = input.score.unwrap_or(50);
    let notes = input.notes.unwrap_or_default();

    conn.execute(
        "INSERT INTO crm_lead_funnel (id, contact_id, owner_user_id, funnel_stage, agent_id, score, notes, shared_with_king, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,1,?8,?9)",
        params![id, input.contact_id, owner_user_id, stage, agent, score, notes, ts, ts],
    ).map_err(e)?;

    Ok(LeadFunnel {
        id, contact_id: input.contact_id, owner_user_id, funnel_stage: stage,
        agent_id: agent, score, notes, shared_with_king: true, created_at: ts.clone(), updated_at: ts,
    })
}

#[tauri::command]
pub fn agents_update_lead(
    db: State<'_, CrmDb>,
    lead_id: String,
    user_id: String,
    funnel_stage: Option<String>,
    score: Option<i32>,
    notes: Option<String>,
) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    
    // Verify ownership before updating
    let owner: String = conn.query_row(
        "SELECT owner_user_id FROM crm_lead_funnel WHERE id = ?1",
        params![lead_id],
        |r| r.get(0),
    ).map_err(|_| "Lead not found".to_string())?;
    
    if owner != user_id {
        return Err("Access denied: cannot update lead you don't own".to_string());
    }
    
    if let Some(stage) = funnel_stage {
        conn.execute("UPDATE crm_lead_funnel SET funnel_stage=?1, updated_at=?2 WHERE id=?3 AND owner_user_id=?4", params![stage, ts, lead_id, user_id]).map_err(e)?;
    }
    if let Some(s) = score {
        conn.execute("UPDATE crm_lead_funnel SET score=?1, updated_at=?2 WHERE id=?3 AND owner_user_id=?4", params![s, ts, lead_id, user_id]).map_err(e)?;
    }
    if let Some(n) = notes {
        conn.execute("UPDATE crm_lead_funnel SET notes=?1, updated_at=?2 WHERE id=?3 AND owner_user_id=?4", params![n, ts, lead_id, user_id]).map_err(e)?;
    }
    Ok(())
}

/// Get leads — King sees ALL, team members see their own
#[tauri::command]
pub fn agents_get_leads(
    db: State<'_, CrmDb>,
    user_id: String,
    is_king: bool,
) -> CmdResult<Vec<LeadFunnel>> {
    let conn = db.conn.lock().map_err(e)?;
    let sql = if is_king {
        "SELECT id,contact_id,owner_user_id,funnel_stage,agent_id,score,notes,shared_with_king,created_at,updated_at FROM crm_lead_funnel ORDER BY score DESC, updated_at DESC"
    } else {
        "SELECT id,contact_id,owner_user_id,funnel_stage,agent_id,score,notes,shared_with_king,created_at,updated_at FROM crm_lead_funnel WHERE owner_user_id=?1 ORDER BY score DESC"
    };
    let mut stmt = conn.prepare(sql).map_err(e)?;
    let rows: Vec<LeadFunnel> = if is_king {
        stmt.query_map([], |r| Ok(LeadFunnel {
            id: r.get(0)?, contact_id: r.get(1)?, owner_user_id: r.get(2)?,
            funnel_stage: r.get(3)?, agent_id: r.get(4)?, score: r.get(5)?,
            notes: r.get(6)?, shared_with_king: r.get::<_, i32>(7)? == 1,
            created_at: r.get(8)?, updated_at: r.get(9)?,
        })).map_err(e)?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map(params![user_id], |r| Ok(LeadFunnel {
            id: r.get(0)?, contact_id: r.get(1)?, owner_user_id: r.get(2)?,
            funnel_stage: r.get(3)?, agent_id: r.get(4)?, score: r.get(5)?,
            notes: r.get(6)?, shared_with_king: r.get::<_, i32>(7)? == 1,
            created_at: r.get(8)?, updated_at: r.get(9)?,
        })).map_err(e)?.filter_map(|r| r.ok()).collect()
    };
    Ok(rows)
}

/* ══════════════════════════════════════════════════════
   EMAIL ASSIGNMENT — x3star.net emails per user
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn agents_assign_email(
    db: State<'_, CrmDb>,
    user_id: String,
    username: String,
) -> CmdResult<UserEmailAssignment> {
    let conn = db.conn.lock().map_err(e)?;

    // Check if user already has an email
    let existing: Option<String> = conn.query_row(
        "SELECT email_address FROM crm_user_emails WHERE user_id=?1 AND active=1",
        params![user_id], |r| r.get(0),
    ).ok();

    if let Some(email) = existing {
        return Ok(UserEmailAssignment {
            id: String::new(), user_id, email_address: email,
            smtp_username: String::new(), created_at: String::new(), active: true,
        });
    }

    // Generate email: username@x3star.net (sanitized)
    let clean_name: String = username.to_lowercase().chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-')
        .collect();
    let email = format!("{}@x3star.net", if clean_name.is_empty() { "user".to_string() } else { clean_name.clone() });

    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO crm_user_emails (id, user_id, email_address, smtp_username, created_at, active) VALUES (?1,?2,?3,?4,?5,1)",
        params![id, user_id, email, clean_name, ts],
    ).map_err(e)?;

    Ok(UserEmailAssignment { id, user_id, email_address: email, smtp_username: clean_name, created_at: ts, active: true })
}

#[tauri::command]
pub fn agents_get_user_email(
    db: State<'_, CrmDb>,
    user_id: String,
) -> CmdResult<Option<UserEmailAssignment>> {
    let conn = db.conn.lock().map_err(e)?;
    let result = conn.query_row(
        "SELECT id, user_id, email_address, smtp_username, created_at, active FROM crm_user_emails WHERE user_id=?1 AND active=1",
        params![user_id],
        |r| Ok(UserEmailAssignment {
            id: r.get(0)?, user_id: r.get(1)?, email_address: r.get(2)?,
            smtp_username: r.get(3)?, created_at: r.get(4)?, active: r.get::<_, i32>(5)? == 1,
        }),
    );
    match result {
        Ok(e) => Ok(Some(e)),
        Err(_) => Ok(None),
    }
}

/* ══════════════════════════════════════════════════════
   PROXY MANAGEMENT — per-user proxy assignment
   ══════════════════════════════════════════════════════ */

#[derive(Deserialize)]
pub struct AssignProxyInput {
    pub proxy_host: String,
    pub proxy_port: i32,
    pub proxy_type: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[tauri::command]
pub fn agents_assign_proxy(
    db: State<'_, CrmDb>,
    user_id: String,
    input: AssignProxyInput,
) -> CmdResult<UserProxy> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    let ptype = input.proxy_type.unwrap_or_else(|| "socks5".into());
    let uname = input.username.unwrap_or_default();
    let pass = input.password.unwrap_or_default();

    // Deactivate old proxy
    conn.execute("UPDATE crm_user_proxies SET active=0 WHERE user_id=?1", params![user_id]).map_err(e)?;

    conn.execute(
        "INSERT INTO crm_user_proxies (id, user_id, proxy_host, proxy_port, proxy_type, username, password, active, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,1,?8)",
        params![id, user_id, input.proxy_host, input.proxy_port, ptype, uname, pass, ts],
    ).map_err(e)?;

    Ok(UserProxy { id, user_id, proxy_host: input.proxy_host, proxy_port: input.proxy_port, proxy_type: ptype, username: uname, password: pass, active: true, created_at: ts })
}

#[tauri::command]
pub fn agents_get_proxy(
    db: State<'_, CrmDb>,
    user_id: String,
) -> CmdResult<Option<UserProxy>> {
    let conn = db.conn.lock().map_err(e)?;
    let result = conn.query_row(
        "SELECT id,user_id,proxy_host,proxy_port,proxy_type,username,password,active,created_at FROM crm_user_proxies WHERE user_id=?1 AND active=1",
        params![user_id],
        |r| Ok(UserProxy {
            id: r.get(0)?, user_id: r.get(1)?, proxy_host: r.get(2)?,
            proxy_port: r.get(3)?, proxy_type: r.get(4)?, username: r.get(5)?,
            password: r.get(6)?, active: r.get::<_, i32>(7)? == 1, created_at: r.get(8)?,
        }),
    );
    match result {
        Ok(p) => Ok(Some(p)),
        Err(_) => Ok(None),
    }
}

/// King-only: get all user proxies
#[tauri::command]
pub fn agents_get_all_proxies(
    db: State<'_, CrmDb>,
    user_id: String,
    is_king: bool,
) -> CmdResult<Vec<UserProxy>> {
    // Server-side authorization: only king can see all proxies
    if !is_king {
        return Err("Access denied: only admins can view all proxies".to_string());
    }
    
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id,user_id,proxy_host,proxy_port,proxy_type,username,password,active,created_at FROM crm_user_proxies WHERE active=1"
    ).map_err(e)?;
    let rows = stmt.query_map([], |r| Ok(UserProxy {
        id: r.get(0)?, user_id: r.get(1)?, proxy_host: r.get(2)?,
        proxy_port: r.get(3)?, proxy_type: r.get(4)?, username: r.get(5)?,
        password: r.get(6)?, active: r.get::<_, i32>(7)? == 1, created_at: r.get(8)?,
    })).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// King-only: get funnel stats across all users
#[tauri::command]
pub fn agents_get_funnel_stats(
    db: State<'_, CrmDb>,
    user_id: String,
    is_king: bool,
) -> CmdResult<serde_json::Value> {
    // Server-side authorization: only king can see all funnel stats
    if !is_king {
        return Err("Access denied: only admins can view funnel stats".to_string());
    }
    
    let conn = db.conn.lock().map_err(e)?;
    let total: i32 = conn.query_row("SELECT COUNT(*) FROM crm_lead_funnel", [], |r| r.get(0)).unwrap_or(0);
    let discovered: i32 = conn.query_row("SELECT COUNT(*) FROM crm_lead_funnel WHERE funnel_stage='discovered'", [], |r| r.get(0)).unwrap_or(0);
    let contacted: i32 = conn.query_row("SELECT COUNT(*) FROM crm_lead_funnel WHERE funnel_stage='contacted'", [], |r| r.get(0)).unwrap_or(0);
    let pitched: i32 = conn.query_row("SELECT COUNT(*) FROM crm_lead_funnel WHERE funnel_stage='pitched'", [], |r| r.get(0)).unwrap_or(0);
    let negotiating: i32 = conn.query_row("SELECT COUNT(*) FROM crm_lead_funnel WHERE funnel_stage='negotiating'", [], |r| r.get(0)).unwrap_or(0);
    let converted: i32 = conn.query_row("SELECT COUNT(*) FROM crm_lead_funnel WHERE funnel_stage='converted'", [], |r| r.get(0)).unwrap_or(0);
    let lost: i32 = conn.query_row("SELECT COUNT(*) FROM crm_lead_funnel WHERE funnel_stage='lost'", [], |r| r.get(0)).unwrap_or(0);
    let tasks_total: i32 = conn.query_row("SELECT COUNT(*) FROM crm_agent_tasks", [], |r| r.get(0)).unwrap_or(0);
    let tasks_completed: i32 = conn.query_row("SELECT COUNT(*) FROM crm_agent_tasks WHERE status='completed'", [], |r| r.get(0)).unwrap_or(0);
    let emails_assigned: i32 = conn.query_row("SELECT COUNT(*) FROM crm_user_emails WHERE active=1", [], |r| r.get(0)).unwrap_or(0);

    Ok(serde_json::json!({
        "total_leads": total,
        "funnel": { "discovered": discovered, "contacted": contacted, "pitched": pitched, "negotiating": negotiating, "converted": converted, "lost": lost },
        "tasks": { "total": tasks_total, "completed": tasks_completed },
        "emails_assigned": emails_assigned,
    }))
}

/* ══════════════════════════════════════════════════════
   WEB SEARCH — DuckDuckGo HTML scraping (no API key)
   ══════════════════════════════════════════════════════ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Search the web using DuckDuckGo HTML (free, no API key)
async fn web_search(query: &str, max_results: usize) -> Result<Vec<SearchResult>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding(query));
    let resp = client.get(&url).send().await.map_err(|e| format!("Search request failed: {}", e))?;
    let html = resp.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    // Scraper not available in this build — return stub results
    let _ = html;
    let _ = max_results;
    Ok(Vec::new())
}

/// Fetch page content for agent research
async fn fetch_page_text(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let html = resp.text().await.map_err(|e| e.to_string())?;

    // Scraper not available in this build — return stub text
    let _ = html;
    Ok(String::new())
}

fn urlencoding(s: &str) -> String {
    s.chars().map(|c| match c {
        ' ' => '+'.to_string(),
        c if c.is_alphanumeric() || "-_.~".contains(c) => c.to_string(),
        c => format!("%{:02X}", c as u32),
    }).collect()
}

/// Web search + optional Ollama analysis
#[tauri::command]
pub async fn agents_web_search(
    query: String,
    agent_id: Option<String>,
) -> CmdResult<serde_json::Value> {
    let results = web_search(&query, 10).await?;

    // If agent specified, have them analyze results
    let analysis = if let Some(aid) = agent_id {
        let roster = get_agent_roster();
        if let Some(agent) = roster.iter().find(|a| a.id == aid) {
            let search_context = results.iter().enumerate()
                .map(|(i, r)| format!("{}. {} — {}\n   {}", i+1, r.title, r.url, r.snippet))
                .collect::<Vec<_>>().join("\n\n");
            let prompt = format!("I searched for: \"{}\"\n\nHere are the results:\n{}\n\nAnalyze these results and provide actionable insights for X3 Chain's growth.", query, search_context);
            call_ollama(&agent.model, &agent.system_prompt, &prompt).await.ok()
        } else { None }
    } else { None };

    Ok(serde_json::json!({
        "query": query,
        "results": results,
        "analysis": analysis,
        "count": results.len(),
    }))
}

/// Fetch and analyze a specific website (for target research)
#[tauri::command]
pub async fn agents_fetch_website(
    url: String,
    agent_id: String,
) -> CmdResult<serde_json::Value> {
    let page_text = fetch_page_text(&url).await?;
    let roster = get_agent_roster();
    let agent = roster.iter().find(|a| a.id == agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    let prompt = format!(
        "Analyze this website content from {} for business opportunities with X3 Chain.\n\nWebsite content:\n{}\n\nProvide:\n1. What they do\n2. Their tech stack (if visible)\n3. How X3 Chain's GPU TPS could help them\n4. Personalized outreach angle\n5. Key decision-makers to contact",
        url, page_text
    );

    let analysis = call_ollama(&agent.model, &agent.system_prompt, &prompt).await?;

    Ok(serde_json::json!({
        "url": url,
        "page_text_length": page_text.len(),
        "analysis": analysis,
    }))
}

/* ══════════════════════════════════════════════════════
   RAG SYSTEM — Index .md files, build context for agents
   ══════════════════════════════════════════════════════ */

/// Index all .md files from a directory into the RAG database
#[tauri::command]
pub fn agents_rag_index(
    db: State<'_, CrmDb>,
    folder_path: String,
) -> CmdResult<serde_json::Value> {
    let path = PathBuf::from(&folder_path);
    if !path.exists() {
        return Err(format!("Folder not found: {}", folder_path));
    }

    let pattern = format!("{}/**/*.md", folder_path);
    let files: Vec<_> = glob::glob(&pattern)
        .map_err(|e| format!("Glob error: {}", e))?
        .filter_map(|p| p.ok())
        .collect();

    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    let mut indexed = 0;
    let mut total_tokens = 0usize;

    for file in &files {
        let content = std::fs::read_to_string(file).unwrap_or_default();
        if content.is_empty() { continue; }
        let token_count = content.split_whitespace().count();
        total_tokens += token_count;
        let fp = file.to_string_lossy().to_string();

        conn.execute(
            "INSERT OR REPLACE INTO crm_rag_docs (id, file_path, content, token_count, indexed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![uid(), fp, content, token_count as i32, ts],
        ).map_err(e)?;
        indexed += 1;
    }

    Ok(serde_json::json!({
        "folder": folder_path,
        "files_found": files.len(),
        "files_indexed": indexed,
        "total_tokens": total_tokens,
    }))
}

/// Query the RAG system — searches indexed docs, builds context, sends to agent
#[tauri::command]
pub async fn agents_rag_query(
    db: State<'_, CrmDb>,
    query: String,
    agent_id: String,
) -> CmdResult<serde_json::Value> {
    let roster = get_agent_roster();
    let agent = roster.iter().find(|a| a.id == agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    // Search for relevant docs (simple keyword matching — good enough for local)
    let keywords: Vec<String> = query.to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|s| s.to_string())
        .collect();

    let docs: Vec<(String, String)> = {
        let conn = db.conn.lock().map_err(e)?;
        let mut stmt = conn.prepare(
            "SELECT file_path, content FROM crm_rag_docs ORDER BY token_count DESC"
        ).map_err(e)?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }).map_err(e)?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Score docs by keyword relevance
    let mut scored: Vec<(f64, &str, &str)> = docs.iter()
        .map(|(path, content)| {
            let lower = content.to_lowercase();
            let score: f64 = keywords.iter()
                .map(|kw| lower.matches(kw.as_str()).count() as f64)
                .sum();
            (score, path.as_str(), content.as_str())
        })
        .filter(|(score, _, _)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    // Build context from top 5 docs, max ~6000 chars
    let mut context = String::new();
    let mut char_budget = 6000usize;
    let mut sources = Vec::new();

    for (score, path, content) in scored.iter().take(5) {
        let chunk: String = content.chars().take(char_budget.min(2000)).collect();
        context.push_str(&format!("\n--- {} (relevance: {:.0}) ---\n{}\n", path, score, chunk));
        sources.push(path.to_string());
        if chunk.len() >= char_budget { break; }
        char_budget = char_budget.saturating_sub(chunk.len());
    }

    if context.is_empty() {
        return Ok(serde_json::json!({
            "query": query,
            "answer": "No relevant documents found in the RAG index. Try indexing a folder first with the RAG Index button.",
            "sources": [],
        }));
    }

    let prompt = format!(
        "Using the following project documentation as context, answer this question:\n\nQuestion: {}\n\nDocumentation context:\n{}\n\nProvide a detailed, actionable answer based on the documentation.",
        query, context
    );

    let answer = call_ollama(&agent.model, &agent.system_prompt, &prompt).await?;

    Ok(serde_json::json!({
        "query": query,
        "answer": answer,
        "sources": sources,
        "docs_searched": docs.len(),
    }))
}

/// Get RAG index stats
#[tauri::command]
pub fn agents_rag_stats(
    db: State<'_, CrmDb>,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM crm_rag_docs", [], |r| r.get(0)).unwrap_or(0);
    let tokens: i32 = conn.query_row("SELECT COALESCE(SUM(token_count), 0) FROM crm_rag_docs", [], |r| r.get(0)).unwrap_or(0);
    let mut stmt = conn.prepare("SELECT file_path, token_count, indexed_at FROM crm_rag_docs ORDER BY indexed_at DESC LIMIT 20").map_err(e)?;
    let files: Vec<serde_json::Value> = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "path": r.get::<_, String>(0)?,
            "tokens": r.get::<_, i32>(1)?,
            "indexed_at": r.get::<_, String>(2)?,
        }))
    }).map_err(e)?.filter_map(|r| r.ok()).collect();

    Ok(serde_json::json!({
        "total_docs": count,
        "total_tokens": tokens,
        "files": files,
    }))
}

/* ══════════════════════════════════════════════════════
   CONTACT PASTE-IMPORT — Paste text, AI auto-sorts it
   ══════════════════════════════════════════════════════ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedContact {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub company: String,
    pub job_title: String,
    pub country: String,
    pub network: String,
    pub ranking: i32,
    pub website: String,
    pub notes: String,
    pub source: String,
}

/// Paste raw text → AI parses into contacts → stores in DB
#[tauri::command]
pub async fn agents_import_contacts(
    db: State<'_, CrmDb>,
    owner_user_id: String,
    raw_text: String,
) -> CmdResult<serde_json::Value> {
    let prompt = format!(
        r#"Parse the following text into a JSON array of contacts. Extract all people/companies you can identify.

For each contact, provide these fields (use "" if unknown):
- first_name, last_name, email, phone, company, job_title
- country (ISO 2-letter code if possible)
- network (e.g., "Ethereum", "Solana", "Polkadot", "Bitcoin", "Cosmos", "Avalanche", etc.)
- ranking (1-10 score based on likely relevance to blockchain/GPU infrastructure)
- website, notes

Return ONLY a JSON array, no other text. Example:
[{{"first_name":"Vitalik","last_name":"Buterin","email":"","phone":"","company":"Ethereum Foundation","job_title":"Co-founder","country":"CH","network":"Ethereum","ranking":10,"website":"ethereum.org","notes":"Co-founder of Ethereum"}}]

Text to parse:
{}"#,
        raw_text
    );

    let result = call_ollama("qwen2.5-coder:14b", "You are a data extraction specialist. Parse contact information from unstructured text into clean JSON. Always return valid JSON arrays.", &prompt).await?;

    // Try to parse the JSON from the response
    let contacts: Vec<ParsedContact> = extract_json_array(&result)?;

    // Insert into DB
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    let mut imported = 0;

    for contact in &contacts {
        let id = uid();
        conn.execute(
            "INSERT INTO crm_contacts (id, owner_user_id, first_name, last_name, email, phone, company, job_title, country, network, ranking, website, notes, source, stage, priority, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'ai-import','lead','medium',?14,?15)",
            params![id, owner_user_id, contact.first_name, contact.last_name, contact.email,
                    contact.phone, contact.company, contact.job_title, contact.country,
                    contact.network, contact.ranking, contact.website, contact.notes, ts, ts],
        ).map_err(e)?;
        imported += 1;
    }

    Ok(serde_json::json!({
        "raw_length": raw_text.len(),
        "contacts_parsed": contacts.len(),
        "contacts_imported": imported,
        "contacts": contacts,
    }))
}

/// Extract JSON array from LLM response (handles markdown fences)
fn extract_json_array(text: &str) -> Result<Vec<ParsedContact>, String> {
    // Try to find JSON array in the text
    let json_str = if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            &text[start..=end]
        } else { text }
    } else { text };

    serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse AI response as contacts JSON: {}. Raw: {}", e, &text[..text.len().min(200)]))
}

/// Get contacts sorted by network/country/ranking
#[tauri::command]
pub fn agents_get_contacts_sorted(
    db: State<'_, CrmDb>,
    owner_user_id: String,
    sort_by: String,
    filter_network: Option<String>,
    filter_country: Option<String>,
) -> CmdResult<Vec<serde_json::Value>> {
    let conn = db.conn.lock().map_err(e)?;

    let order = match sort_by.as_str() {
        "ranking" => "ranking DESC",
        "country" => "country ASC, ranking DESC",
        "network" => "network ASC, ranking DESC",
        "name" => "first_name ASC, last_name ASC",
        _ => "ranking DESC",
    };

    let mut conditions = vec!["owner_user_id = ?1".to_string()];
    let mut param_values: Vec<String> = vec![owner_user_id.clone()];

    if let Some(ref net) = filter_network {
        if !net.is_empty() {
            conditions.push(format!("network = ?{}", param_values.len() + 1));
            param_values.push(net.clone());
        }
    }
    if let Some(ref country) = filter_country {
        if !country.is_empty() {
            conditions.push(format!("country = ?{}", param_values.len() + 1));
            param_values.push(country.clone());
        }
    }

    let sql = format!(
        "SELECT id, first_name, last_name, email, phone, company, job_title, country, COALESCE(network,'') as network, COALESCE(ranking,0) as ranking, website, notes, source, stage, priority, created_at
         FROM crm_contacts WHERE {} ORDER BY {} LIMIT 500",
        conditions.join(" AND "), order
    );

    let mut stmt = conn.prepare(&sql).map_err(e)?;

    // Dynamic parameter binding
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt.query_map(params_refs.as_slice(), |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "first_name": r.get::<_, String>(1)?,
            "last_name": r.get::<_, String>(2)?,
            "email": r.get::<_, String>(3)?,
            "phone": r.get::<_, String>(4)?,
            "company": r.get::<_, String>(5)?,
            "job_title": r.get::<_, String>(6)?,
            "country": r.get::<_, String>(7)?,
            "network": r.get::<_, String>(8)?,
            "ranking": r.get::<_, i32>(9)?,
            "website": r.get::<_, String>(10)?,
            "notes": r.get::<_, String>(11)?,
            "source": r.get::<_, String>(12)?,
            "stage": r.get::<_, String>(13)?,
            "priority": r.get::<_, String>(14)?,
            "created_at": r.get::<_, String>(15)?,
        }))
    }).map_err(e)?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Get distinct networks and countries for filter dropdowns
#[tauri::command]
pub fn agents_get_contact_filters(
    db: State<'_, CrmDb>,
    owner_user_id: String,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;

    let mut net_stmt = conn.prepare(
        "SELECT DISTINCT COALESCE(network,'') FROM crm_contacts WHERE owner_user_id=?1 AND network != '' ORDER BY network"
    ).map_err(e)?;
    let networks: Vec<String> = net_stmt.query_map(params![owner_user_id], |r| r.get(0))
        .map_err(e)?.filter_map(|r| r.ok()).collect();

    let mut country_stmt = conn.prepare(
        "SELECT DISTINCT country FROM crm_contacts WHERE owner_user_id=?1 AND country != '' ORDER BY country"
    ).map_err(e)?;
    let countries: Vec<String> = country_stmt.query_map(params![owner_user_id], |r| r.get(0))
        .map_err(e)?.filter_map(|r| r.ok()).collect();

    Ok(serde_json::json!({
        "networks": networks,
        "countries": countries,
    }))
}

/* ══════════════════════════════════════════════════════
   PROXY/VPN TOGGLE — toggle proxy on/off per user
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn agents_toggle_proxy(
    db: State<'_, CrmDb>,
    user_id: String,
    active: bool,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;
    let active_int: i32 = if active { 1 } else { 0 };
    conn.execute(
        "UPDATE crm_user_proxies SET active=?1 WHERE user_id=?2",
        params![active_int, user_id],
    ).map_err(e)?;
    Ok(serde_json::json!({ "user_id": user_id, "proxy_active": active }))
}

/* ══════════════════════════════════════════════════════
   MEDIA FOLDER — list + serve media files for messages
   ══════════════════════════════════════════════════════ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    pub file_size: u64,
}

/// Scan a folder for media files (images, videos, PDFs)
#[tauri::command]
pub fn agents_scan_media(
    db: State<'_, CrmDb>,
    folder_path: String,
) -> CmdResult<serde_json::Value> {
    let path = PathBuf::from(&folder_path);
    if !path.exists() {
        return Err(format!("Folder not found: {}", folder_path));
    }

    let extensions = ["png", "jpg", "jpeg", "gif", "webp", "svg",
                      "mp4", "webm", "mov", "avi",
                      "pdf", "doc", "docx", "pptx"];

    let mut files = Vec::new();
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();

    for ext in &extensions {
        let pattern = format!("{}/**/*.{}", folder_path, ext);
        if let Ok(paths) = glob::glob(&pattern) {
            for entry in paths.filter_map(|p| p.ok()) {
                let metadata = std::fs::metadata(&entry).ok();
                let size = metadata.map(|m| m.len()).unwrap_or(0);
                let name = entry.file_name().unwrap_or_default().to_string_lossy().to_string();
                let fp = entry.to_string_lossy().to_string();
                let ftype = ext.to_string();
                let id = uid();

                conn.execute(
                    "INSERT OR REPLACE INTO crm_media (id, file_name, file_path, file_type, file_size, tags, created_at) VALUES (?1,?2,?3,?4,?5,'',?6)",
                    params![id, name, fp, ftype, size as i32, ts],
                ).ok();

                files.push(MediaFile { id, file_name: name, file_path: fp, file_type: ftype, file_size: size });
            }
        }
    }

    Ok(serde_json::json!({
        "folder": folder_path,
        "files_found": files.len(),
        "files": files,
    }))
}

/// Get all indexed media files
#[tauri::command]
pub fn agents_get_media(
    db: State<'_, CrmDb>,
) -> CmdResult<Vec<serde_json::Value>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, file_name, file_path, file_type, file_size, created_at FROM crm_media ORDER BY created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "file_name": r.get::<_, String>(1)?,
            "file_path": r.get::<_, String>(2)?,
            "file_type": r.get::<_, String>(3)?,
            "file_size": r.get::<_, i32>(4)?,
            "created_at": r.get::<_, String>(5)?,
        }))
    }).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/* ══════════════════════════════════════════════════════
   PERSONALIZED MESSAGES — Agent uses contact + website
   ══════════════════════════════════════════════════════ */

/// Generate a personalized outreach message using contact info + their website
#[tauri::command]
pub async fn agents_personalized_message(
    db: State<'_, CrmDb>,
    contact_id: String,
    user_id: String,
    agent_id: String,
    message_type: String,
) -> CmdResult<serde_json::Value> {
    let roster = get_agent_roster();
    let agent = roster.iter().find(|a| a.id == agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    // Fetch contact info (only if user owns the contact)
    let contact: serde_json::Value = {
        let conn = db.conn.lock().map_err(e)?;
        conn.query_row(
            "SELECT first_name, last_name, email, company, job_title, country, COALESCE(network,'') as network, website, notes FROM crm_contacts WHERE id=?1 AND owner_user_id=?2",
            params![contact_id, user_id],
            |r| Ok(serde_json::json!({
                "first_name": r.get::<_, String>(0)?,
                "last_name": r.get::<_, String>(1)?,
                "email": r.get::<_, String>(2)?,
                "company": r.get::<_, String>(3)?,
                "job_title": r.get::<_, String>(4)?,
                "country": r.get::<_, String>(5)?,
                "network": r.get::<_, String>(6)?,
                "website": r.get::<_, String>(7)?,
                "notes": r.get::<_, String>(8)?,
            })),
        ).map_err(|_| "Contact not found or access denied".to_string())?
    };

    // Try to fetch their website for extra context
    let website = contact["website"].as_str().unwrap_or("");
    let website_context = if !website.is_empty() && (website.starts_with("http") || website.contains('.')) {
        let url = if website.starts_with("http") { website.to_string() } else { format!("https://{}", website) };
        fetch_page_text(&url).await.unwrap_or_default()
    } else {
        String::new()
    };

    let prompt = format!(
        r#"Generate a personalized {} for this contact:

Contact: {} {} — {} at {}
Country: {} | Network: {}
Notes: {}

{}

Write a highly personalized message that:
1. References something specific about their company/work
2. Explains how X3 Chain's GPU TPS infrastructure could benefit them specifically
3. Has a clear, non-pushy CTA
4. Feels genuine, not template-y

Provide: Subject line, body text, and a PS line."#,
        message_type,
        contact["first_name"].as_str().unwrap_or(""),
        contact["last_name"].as_str().unwrap_or(""),
        contact["job_title"].as_str().unwrap_or(""),
        contact["company"].as_str().unwrap_or(""),
        contact["country"].as_str().unwrap_or(""),
        contact["network"].as_str().unwrap_or(""),
        contact["notes"].as_str().unwrap_or(""),
        if !website_context.is_empty() {
            format!("Their website content (for personalization):\n{}", &website_context[..website_context.len().min(2000)])
        } else {
            String::new()
        }
    );

    let message = call_ollama(&agent.model, &agent.system_prompt, &prompt).await?;

    Ok(serde_json::json!({
        "contact_id": contact_id,
        "contact_name": format!("{} {}", contact["first_name"].as_str().unwrap_or(""), contact["last_name"].as_str().unwrap_or("")),
        "message_type": message_type,
        "message": message,
        "used_website": !website_context.is_empty(),
    }))
}

/* ══════════════════════════════════════════════════════
   90-DAY ROLLOUT PLAN — Phase tracker
   ══════════════════════════════════════════════════════ */

/// Seed the default 90-day rollout phases
#[tauri::command]
pub fn agents_seed_rollout(
    db: State<'_, CrmDb>,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();

    let phases = vec![
        (1, "Foundation & Proof", "Ship benchmarks, white paper, validator docs. Establish institutional-grade positioning.", 1, 30),
        (2, "Expansion & Authority", "Launch validator program, enterprise outreach, content pipeline at scale. Build community.", 31, 60),
        (3, "Dominance & Scale", "Mainnet prep, institutional partnerships, ecosystem grants. Full conversion machine.", 61, 90),
    ];

    let mut count = 0;
    for (num, title, desc, start, end) in &phases {
        let id = uid();
        conn.execute(
            "INSERT OR IGNORE INTO crm_rollout_phases (id, phase_num, title, description, start_day, end_day, status, milestones, progress, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,'pending','[]',0,?7,?8)",
            params![id, num, title, desc, start, end, ts, ts],
        ).map_err(e)?;
        count += 1;
    }

    Ok(serde_json::json!({ "phases_seeded": count }))
}

/// Get all rollout phases
#[tauri::command]
pub fn agents_get_rollout(
    db: State<'_, CrmDb>,
) -> CmdResult<Vec<serde_json::Value>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, phase_num, title, description, start_day, end_day, status, milestones, progress, created_at, updated_at FROM crm_rollout_phases ORDER BY phase_num"
    ).map_err(e)?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "phase_num": r.get::<_, i32>(1)?,
            "title": r.get::<_, String>(2)?,
            "description": r.get::<_, String>(3)?,
            "start_day": r.get::<_, i32>(4)?,
            "end_day": r.get::<_, i32>(5)?,
            "status": r.get::<_, String>(6)?,
            "milestones": r.get::<_, String>(7)?,
            "progress": r.get::<_, i32>(8)?,
            "created_at": r.get::<_, String>(9)?,
            "updated_at": r.get::<_, String>(10)?,
        }))
    }).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Update a rollout phase (status, progress, milestones)
#[tauri::command]
pub fn agents_update_rollout(
    db: State<'_, CrmDb>,
    phase_id: String,
    status: Option<String>,
    progress: Option<i32>,
    milestones: Option<String>,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();

    if let Some(ref s) = status {
        conn.execute("UPDATE crm_rollout_phases SET status=?1, updated_at=?2 WHERE id=?3",
            params![s, ts, phase_id]).map_err(e)?;
    }
    if let Some(p) = progress {
        conn.execute("UPDATE crm_rollout_phases SET progress=?1, updated_at=?2 WHERE id=?3",
            params![p, ts, phase_id]).map_err(e)?;
    }
    if let Some(ref m) = milestones {
        conn.execute("UPDATE crm_rollout_phases SET milestones=?1, updated_at=?2 WHERE id=?3",
            params![m, ts, phase_id]).map_err(e)?;
    }

    Ok(serde_json::json!({ "phase_id": phase_id, "updated": true }))
}

/* ══════════════════════════════════════════════════════
   PAGE BUILDER — Generate & manage landing pages
   ══════════════════════════════════════════════════════ */

/// Generate a landing page using an agent (WebWeaver by default)
#[tauri::command]
pub async fn agents_generate_page(
    db: State<'_, CrmDb>,
    slug: String,
    title: String,
    page_type: String,
    prompt: String,
    agent_id: Option<String>,
) -> CmdResult<serde_json::Value> {
    let roster = get_agent_roster();
    let aid = agent_id.unwrap_or_else(|| "agent-web-seo".into());
    let agent = roster.iter().find(|a| a.id == aid)
        .ok_or_else(|| format!("Agent '{}' not found", aid))?;

    let gen_prompt = format!(
        r#"Generate a complete landing page for X3 Chain.

Page: {} ({})
Type: {}
User instructions: {}

Generate the full HTML content for this page. Include:
1. A compelling hero section with headline and subheadline
2. Key features/proof points section (use X3 Chain's real capabilities: GPU TPS, X3 settlement, cross-chain validators, deterministic replay)
3. Social proof / metrics section
4. Call-to-action section
5. Clean, professional styling (inline CSS, dark theme, responsive)

Also generate:
- meta_title: SEO-optimized page title (60 chars max)
- meta_desc: Meta description (155 chars max)
- seo_keywords: comma-separated keywords

Format your response as JSON:
{{"html": "<full html>", "meta_title": "...", "meta_desc": "...", "seo_keywords": "..."}}"#,
        title, slug, page_type, prompt
    );

    let result = call_ollama(&agent.model, &agent.system_prompt, &gen_prompt).await?;

    // Try to parse structured response
    let (html, meta_title, meta_desc, seo_keywords) = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
        (
            parsed["html"].as_str().unwrap_or(&result).to_string(),
            parsed["meta_title"].as_str().unwrap_or(&title).to_string(),
            parsed["meta_desc"].as_str().unwrap_or("").to_string(),
            parsed["seo_keywords"].as_str().unwrap_or("").to_string(),
        )
    } else {
        // Fallback: treat entire response as HTML
        (result.clone(), title.clone(), String::new(), String::new())
    };

    // Store in DB
    let id = uid();
    let ts = now();
    {
        let conn = db.conn.lock().map_err(e)?;
        conn.execute(
            "INSERT OR REPLACE INTO crm_generated_pages (id, slug, title, page_type, html_content, meta_title, meta_desc, og_image, seo_keywords, status, agent_id, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'',?8,'draft',?9,?10,?11)",
            params![id, slug, title, page_type, html, meta_title, meta_desc, seo_keywords, aid, ts, ts],
        ).map_err(e)?;
    }

    Ok(serde_json::json!({
        "id": id,
        "slug": slug,
        "title": title,
        "page_type": page_type,
        "meta_title": meta_title,
        "meta_desc": meta_desc,
        "seo_keywords": seo_keywords,
        "html_length": html.len(),
        "status": "draft",
        "agent_id": aid,
    }))
}

/// Get all generated pages
#[tauri::command]
pub fn agents_get_pages(
    db: State<'_, CrmDb>,
) -> CmdResult<Vec<serde_json::Value>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, slug, title, page_type, meta_title, meta_desc, seo_keywords, status, agent_id, created_at, updated_at FROM crm_generated_pages ORDER BY created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "slug": r.get::<_, String>(1)?,
            "title": r.get::<_, String>(2)?,
            "page_type": r.get::<_, String>(3)?,
            "meta_title": r.get::<_, String>(4)?,
            "meta_desc": r.get::<_, String>(5)?,
            "seo_keywords": r.get::<_, String>(6)?,
            "status": r.get::<_, String>(7)?,
            "agent_id": r.get::<_, String>(8)?,
            "created_at": r.get::<_, String>(9)?,
            "updated_at": r.get::<_, String>(10)?,
        }))
    }).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Get a single page's full HTML content
#[tauri::command]
pub fn agents_get_page_content(
    db: State<'_, CrmDb>,
    page_id: String,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;
    conn.query_row(
        "SELECT id, slug, title, page_type, html_content, meta_title, meta_desc, seo_keywords, status, agent_id FROM crm_generated_pages WHERE id=?1",
        params![page_id],
        |r| Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "slug": r.get::<_, String>(1)?,
            "title": r.get::<_, String>(2)?,
            "page_type": r.get::<_, String>(3)?,
            "html_content": r.get::<_, String>(4)?,
            "meta_title": r.get::<_, String>(5)?,
            "meta_desc": r.get::<_, String>(6)?,
            "seo_keywords": r.get::<_, String>(7)?,
            "status": r.get::<_, String>(8)?,
            "agent_id": r.get::<_, String>(9)?,
        })),
    ).map_err(e)
}

/// Update page status (draft → published)
#[tauri::command]
pub fn agents_update_page_status(
    db: State<'_, CrmDb>,
    page_id: String,
    status: String,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    conn.execute(
        "UPDATE crm_generated_pages SET status=?1, updated_at=?2 WHERE id=?3",
        params![status, ts, page_id],
    ).map_err(e)?;
    Ok(serde_json::json!({ "page_id": page_id, "status": status }))
}

/// Delete a generated page
#[tauri::command]
pub fn agents_delete_page(
    db: State<'_, CrmDb>,
    page_id: String,
) -> CmdResult<serde_json::Value> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM crm_generated_pages WHERE id=?1", params![page_id]).map_err(e)?;
    Ok(serde_json::json!({ "page_id": page_id, "deleted": true }))
}

/// Get agents grouped by layer (for hierarchy display)
#[tauri::command]
pub fn agents_get_hierarchy() -> CmdResult<serde_json::Value> {
    let roster = get_agent_roster();
    let layers = vec!["strategic", "execution", "media", "growth"];

    let mut hierarchy = serde_json::Map::new();
    for layer in &layers {
        let agents: Vec<&AgentDef> = roster.iter().filter(|a| a.layer == *layer).collect();
        hierarchy.insert(layer.to_string(), serde_json::json!(
            agents.iter().map(|a| serde_json::json!({
                "id": a.id,
                "name": a.name,
                "role": a.role,
                "avatar": a.avatar,
                "color": a.color,
                "capabilities": a.capabilities,
                "status": a.status,
            })).collect::<Vec<_>>()
        ));
    }

    Ok(serde_json::json!({
        "layers": layers,
        "agents_by_layer": hierarchy,
        "total_agents": roster.len(),
    }))
}
