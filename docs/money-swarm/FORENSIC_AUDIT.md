# Money Swarm V4 forensic audit

Baseline: fetched master `70423054e2d9560a212ab763d4f265f9cdc448d9`, inspected
2026-10-18. See [grant/compute audit](../grant-compute/CURRENT_STATE.md) for
branch/PR/CI/infrastructure evidence and [V2/V3 traces](../grant-compute/V2_FORENSIC_AUDIT.md)
for API-to-storage paths. This extends the same mission; V1–V3 are not assumed
complete or deployed.

## Additional V4 findings

`apps/x3-desktop/src-tauri/migrations/hardware_acquisition.sql` defines hardware
sources, units, campaigns, shipments, inventory and outreach records. Before
reusing it, prove the migration runner and command wiring: a schema file alone
is not a durable working feature. `hardware_acquisition_commands.rs` constructs
new campaign/source/unit objects, without database persistence in the inspected
creation functions. Its inventory/revenue models are not measured infrastructure.

`apps/x3-desktop/src-tauri/src/crm/hardware_sources_db.rs` contains hard-coded
personal contacts, annual values and success percentages without source provenance.
They are unverified; do not import them as real contacts, public sponsorship
offers, success probabilities or expected revenue. Existence of a manufacturer
does not verify a named employee or donation program. Inspect reachability before
modifying legacy paths. `outreach_system.rs` and `smtp.rs` are separate existing
communications code; no outreach was sent during this mission.

`grants/GRANTS_DATABASE.md` is a historical research list, with dated status
labels and a mixture of grants, credits, discounts and paid marketplaces. It is
an input for source re-verification, not an authoritative current opportunity
database and not evidence of X3 eligibility. Do not pad a 200-target requirement
using unverified contacts or duplicate listings.

`scripts/x3-cluster/inventory.json`, `scripts/gpu/gpu_inventory.sh`, cluster audit
artifacts and `tools/launchops/src/inventory.rs` provide existing inventory
discovery/evidence designs. Historical JSON is not live inventory. The supplied
10-server list, switches, storage arrays and UPS equipment remain historical
until read-only device-specific collection verifies them. Do not publish serial
numbers or management credentials. Local host resources were inspected without
changing configuration; x3gpu1 SSH was unreachable and x3star2/x3gpu2 roles remain
unverified.

`pallets/x3-inventory` is on-chain domain code; it must not become a public store
for physical equipment serials or confidential sponsor records. The orchestra
control-plane `crm.rs` trait concerns electorate/vote import, not a funding CRM.
Reuse infrastructure where semantics match, rather than assuming equivalent
names mean equivalent services.

## Priority and implementation boundaries

First fix false funding success and bypassed approval transitions in the gateway.
Then build normalized source/opportunity snapshots that distinguish cash,
equipment grants, permanent donations, discounts, research loans, surplus
purchases and credits. Unknown value, eligibility and availability remain unknown.
Retain the established PostgreSQL migration framework and exact-head CI; add
shared migrations to gateway and indexer together.

Add source-verified hardware discovery before requests or outreach. Match requests
against verified inventory, compatibility and operating costs; a free older server
may increase energy expenses. Power and cooling inputs must declare measured vs
estimated values; solar offset is unknown without readings. Manufacturer-specific
packages remain drafts until reviewed. Suppression and per-message approval must
precede any sender integration. Legal entity, tax, SAM/UEI, banking and partner
records remain private and unverified until documentation supports them.

The existing legacy shell worker cannot enroll volunteers. GrantProof/SwarmProof
packages must reference real tests, verified source snapshots and actual execution
records. No awards, sponsor acceptances, acquired equipment, compensation, deployed
GPU kernels or production readiness are claimed by this audit.
