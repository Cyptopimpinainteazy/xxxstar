# X3 Outbound Policy

*Rules governing everything the X3 system may publish, broadcast, send, or post to any external surface. This policy applies to all agents, tools, automation pipelines, and operator-assisted actions. Nothing goes outbound without a matching rule that permits it.*

**Status:** approval tier model is implemented in `crates/x3-swarm-core/src/approval.rs` and `src/policy.rs`. A dedicated publishing gate service and content provenance pipeline are planned. Until those exist, outbound actions require explicit operator sign-off on every item.

---

## Governing principle

**The default is silence.** No content, message, post, direct contact, or public statement may leave the system without an explicit policy rule that authorises it, a matching approval record, and an operator-attributable identity attached.

Automation may assist. Automation may not decide.

---

## Surface definitions

| Surface | Examples | Policy tier |
|---|---|---|
| Social media (public post) | Twitter/X, Farcaster, LinkedIn, YouTube | Operator-approved publish |
| Forum or community post | Discord, Telegram, Reddit, GitHub Discussions | Operator-approved publish |
| Direct message (individual) | DM to a specific person | Operator-approved outreach |
| Email outreach | Cold email, newsletter | Operator-approved outreach |
| Press or media | Press release, interview, quote | Founder-mediated only |
| Partner or investor communication | Deck, term sheet, LOI | Founder-mediated only |
| On-chain announcement | Governance proposal, on-chain message | Governance approval |
| Documentation (public) | docs site, GitHub wiki | Operator-approved publish |
| Internal-only | Team Slack, internal notes, draft folder | No approval required |

---

## Policy tiers

### Tier 0 — Blocked

Never allowed regardless of approval state:

1. Posting as a real human identity without explicit disclosure that the content is AI-assisted or system-generated
2. Impersonating a team member, partner, investor, or any named individual
3. Claiming achievements, metrics, or capabilities that are not verifiably true
4. Fabricated testimonials, fake reviews, or synthetic social proof
5. Unsolicited direct messages to individuals who have not opted in
6. Autonomous relationship maintenance or follow-up without operator instruction
7. Any outbound action that would move capital, commit resources, or make binding representations without signed operator authorisation
8. Posting on surfaces that prohibit automation without required disclosure

Tier 0 violations by an agent are Class C misconduct (see [AGENT_LAW.md](../swarm-governance/AGENT_LAW.md) §3.1).

---

### Tier 1 — Operator-approved publish

**Applies to:** public posts, forum posts, documentation updates, community replies.

Requirements before any item is published:
- Draft created and stored in `reports/content/` or approved staging path
- Asset carries content provenance metadata (source material, model/tool path, editor history, creation date)
- Human operator has reviewed and explicitly approved the item
- Approval record exists in the swarm approval system (`crates/x3-swarm-core/src/approval.rs`) with: approver identity, approval timestamp, content hash
- Disclosure statement included where platform policy or audience expectations require it
- Campaign owner is recorded; content is attributable to a real operator or founder identity

**Agents permitted to assist:** `Marketing`, `FeatureMapper`, `Grant` (for draft creation only). No agent may submit or post without the approval record being finalised.

**What agents may not do under Tier 1:**
- Schedule or auto-post after approval expires (stale approvals must be re-approved)
- Modify content after approval without new approval record
- Respond to replies or comments autonomously

---

### Tier 2 — Operator-approved outreach

**Applies to:** direct messages, cold email, partnership contact.

Requirements before any outreach is sent:
- All Tier 1 requirements
- A do-not-contact list has been checked for the recipient
- The recipient has not previously requested no-contact
- The outreach is directed at a professional or institutional contact, not a private individual
- A campaign owner is named and takes personal responsibility for the contact
- Volume limits are respected: maximum 5 new outreach contacts per day per campaign without operator expansion approval

**Agents permitted to assist:** `Marketing` (draft only), `Grant` (grant-specific outreach only). No agent may send or submit.

**Prohibited:**
- Bulk cold outreach above volume limits
- Outreach to individuals who have publicly requested not to be contacted
- Any contact that could be construed as harassment or spam under applicable platform rules or law

---

### Tier 3 — Founder-mediated only

**Applies to:** press, media, investor communication, partner negotiation.

These surfaces are not available to any automated system. The system may assist with:
- Draft preparation
- Talking point extraction from existing approved content
- Transcript repurposing into draft form
- Multilingual variants of approved source material

The founder or authorised spokesperson must personally review, edit, and send all content on Tier 3 surfaces. No approval record in the swarm system constitutes permission to send on these surfaces.

---

## Content provenance requirements

Every asset created for potential publication must carry:

| Field | Description |
|---|---|
| `source_material` | The approved source (transcript, document, prior post) this builds on |
| `tool_path` | Models or tools used to generate or transform the content |
| `editor_history` | Who edited after generation and what changed |
| `usage_rights` | Any third-party content rights or attribution requirements |
| `approval_state` | `draft` / `approved` / `published` / `rejected` |
| `campaign_owner` | The human accountable for this content |
| `created_at` | Timestamp |
| `content_hash` | SHA-256 hash of final approved content |

**Status:** provenance metadata format is specified here. Storage in `reports/content/` and integration with the approval system is planned.

---

## Disclosure rules

When any output is AI-assisted or system-generated, disclosure is required if:

1. The platform's terms of service require it
2. The audience would reasonably assume the content is fully human-authored
3. The content touches medical, legal, financial, or safety topics
4. The content is presented as a personal experience or first-person testimonial

Disclosure examples:
- Post caption: *(AI-assisted draft, reviewed and approved by [Name])*
- Document footer: *This document was drafted with AI assistance and reviewed by the X3 team.*

Omitting required disclosure is a Class C misconduct violation for the approving operator and the drafting agent.

---

## Do-not-contact list

The system must maintain a do-not-contact register. Any individual or organisation on this list must never receive direct outreach, regardless of campaign owner or approval tier. The list is append-only. Entries may not be removed except by a named operator with documented justification.

**Status:** planned. No do-not-contact service is implemented yet.

---

## Campaign memory and performance tracking

Every published item should be tracked for:
- Approval history
- Publication timestamp and surface
- Performance signal (reach, engagement, conversion — where accessible)
- Rejection reason if applicable
- Operator feedback

The learning loop should optimise for signal quality and operator trust, not raw post volume.

**Status:** planned. See §8.5 of the master plan.

---

## Audit trail

Every outbound action must produce:
- Approval record with content hash
- Publication confirmation with platform receipt or timestamp
- Operator sign-off identity
- Any post-publication modifications flagged as separate approval records

This log must be retained for a minimum of 90 days.

---

## Enforcement

Until a dedicated publishing gate service is built:

1. All Tier 1 and Tier 2 actions require manual operator execution after swarm-generated approval record
2. No agent is permitted to hold platform credentials or API tokens for publishing surfaces
3. Credentials for external platforms must be held by a human operator only

Once the publishing gate service is implemented, it must:
- Enforce policy tier checks before allowing any API call to an external surface
- Require a valid, unexpired approval record
- Log every outbound action to the audit trail
- Block any call to a Tier 0 surface unconditionally

---

## Open gaps (as of 2026-05-08)

| Gap | Priority |
|---|---|
| Publishing gate service | Band 1 |
| Content provenance storage | Band 1 |
| Do-not-contact register | Band 1 |
| Campaign memory and performance feed | Band 2 |
| Automated disclosure injection | Band 2 |
