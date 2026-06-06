## ADDED Requirements

### Requirement: Single authoritative operator dashboard
The system SHALL provide a single, multi-page operator dashboard branded as Inferstructor.

#### Scenario: Multi-page navigation
- **WHEN** an operator navigates the dashboard
- **THEN** they can access Overview, Validators, Swaps, Proofs, Faucet, Funding, and Settings without dead ends

### Requirement: Grounded copy and data-first UI
The dashboard SHALL use professional, non‑exaggerated copy and prioritize data-driven views.

#### Scenario: Copy review
- **WHEN** a page renders
- **THEN** its primary copy is operational and avoids speculative language

### Requirement: Admin control surface
The dashboard SHALL provide controls for validator approval, chain onboarding, RPC endpoints, faucet limits, and emergency pause.

#### Scenario: Emergency pause
- **WHEN** an admin triggers emergency pause
- **THEN** the system records the action and exposes the paused state in the UI
