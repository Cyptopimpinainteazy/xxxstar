# Jury System Documentation

## Overview

The off-chain jury system provides a mechanism for governing major tasks and proposals in X3 Chain. It implements a commit-reveal voting protocol ensuring vote anonymity while maintaining audit trails for forensic analysis.

## Architecture

### Components

1. **JuryManager** (`swarm/jury/manager.py`)
   - Manages jury session lifecycle
   - Implements commit-reveal voting protocol
   - Enforces quorum and majority rules
   - Supports jury rotation from on-chain agents

2. **AuditLogger** (`swarm/jury/audit.py`)
   - Records all jury activities
   - Computes integrity hashes (SHA256)
   - Prepares logs for on-chain anchoring
   - Provides audit trail retrieval

3. **API Endpoints** (`swarm/api_server.py`)
   - REST API for jury operations
   - Session management
   - Vote submission (commit/reveal)
   - Results aggregation
   - Audit trail retrieval

### Data Flow

```
[Task Intent] → [Severity Classification] → [Jury Session Created]
                                              ↓
                                         [Commit Phase]
                                              ↓
                                         [Reveal Phase]
                                              ↓
                                         [Vote Aggregation]
                                              ↓
                                    [Audit Log Sealed]
                                              ↓
                                  [On-Chain Anchor Hash]
```

## Voting Protocol

### Commit-Reveal Process

The jury uses a two-phase voting protocol to prevent vote coercion and collusion:

#### Phase 1: Commit (5 minutes default)
1. Each jury member generates a vote (True/False)
2. Member creates a nonce (random secret)
3. Member computes commitment: `C = SHA256(vote || nonce)`
4. Member submits commitment (not the vote itself)

#### Phase 2: Reveal (5 minutes default)
1. Commit phase deadline passes
2. Transition to reveal phase triggered
3. Each jury member submits: `(vote, nonce)`
4. System verifies: `SHA256(vote || nonce) == stored commitment`
5. Votes are tallied anonymously

#### Aggregation
- Count yes/no votes
- Apply quorum rule: 66% majority + minimum 3 members
- Publish only aggregate outcome
- Archive encrypted detailed log

### Example: Happy Path

```python
import hashlib
from swarm.jury import JuryManager
from swarm.jury.manager import JuryMember

# Create jury session
jury_manager = JuryManager()
members = [
    JuryMember(agent_id="juror-1", section="governance", is_on_chain=False),
    JuryMember(agent_id="juror-2", section="security", is_on_chain=False),
    JuryMember(agent_id="juror-3", section="economics", is_on_chain=False),
]

session = jury_manager.create_session(
    task_ids=["task-1"],
    members=members,
    commit_timeout_s=300,
    reveal_timeout_s=300
)

print(f"Created session: {session.session_id}")
print(f"State: {session.state.value}")  # "commit"

# COMMIT PHASE: Each juror submits a commitment
for juror in members:
    vote = True  # Vote to approve
    nonce = f"secret-{juror.agent_id}"
    commitment = hashlib.sha256((str(int(vote)) + "|" + nonce).encode()).hexdigest()
    
    ok = jury_manager.submit_commit(session.session_id, juror.agent_id, commitment)
    print(f"{juror.agent_id} committed: {ok}")

# ADVANCE TO REVEAL PHASE
ok = jury_manager.advance_to_reveal(session.session_id)
print(f"Advanced to reveal: {ok}")

# REVEAL PHASE: Each juror reveals their vote
for juror in members:
    vote = True
    nonce = f"secret-{juror.agent_id}"
    
    ok = jury_manager.submit_reveal(session.session_id, juror.agent_id, vote, nonce)
    print(f"{juror.agent_id} revealed: {ok}")

# AGGREGATE VOTES
result = jury_manager.aggregate(session.session_id)
print(f"Results: {result}")
# Output:
# {
#     'yes': 3,
#     'no': 0,
#     'total': 3,
#     'quorum_met': True,
#     'result': True  # APPROVED
# }
```

## API Usage

### Creating a Jury Session

```bash
curl -X POST http://localhost:8080/api/jury/session \
  -H "Content-Type: application/json" \
  -d '{
    "task_ids": ["task-1"],
    "members": [
      {"agent_id": "juror-1", "section": "governance"},
      {"agent_id": "juror-2", "section": "security"},
      {"agent_id": "juror-3", "section": "economics"}
    ]
  }'
```

Response:
```json
{
  "success": true,
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "state": "commit",
  "jury_size": 3,
  "commit_deadline": 1707123456.789,
  "reveal_deadline": 1707124256.789
}
```

### Submitting a Vote Commitment

```bash
curl -X POST http://localhost:8080/api/jury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "type": "commit",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "member_id": "juror-1",
    "commitment": "abc123def456..."
  }'
```

### Advancing to Reveal Phase

```bash
curl -X POST http://localhost:8080/api/jury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "type": "advance",
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

### Revealing a Vote

```bash
curl -X POST http://localhost:8080/api/jury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "type": "reveal",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "member_id": "juror-1",
    "vote": true,
    "nonce": "secret"
  }'
```

### Aggregating Votes

```bash
curl -X POST http://localhost:8080/api/jury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "type": "aggregate",
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

Response:
```json
{
  "success": true,
  "outcome": "APPROVED",
  "result": {
    "yes": 3,
    "no": 0,
    "total": 3,
    "quorum_met": true,
    "result": true
  }
}
```

### Retrieving Session Details

```bash
curl http://localhost:8080/api/jury/session/550e8400-e29b-41d4-a716-446655440000
```

Response:
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "state": "completed",
  "task_ids": ["task-1"],
  "jury": [
    {
      "agent_id": "juror-1",
      "section": "governance",
      "is_on_chain": false,
      "vote_status": "revealed"
    },
    {
      "agent_id": "juror-2",
      "section": "security",
      "is_on_chain": false,
      "vote_status": "revealed"
    },
    {
      "agent_id": "juror-3",
      "section": "economics",
      "is_on_chain": false,
      "vote_status": "revealed"
    }
  ],
  "created_at": 1707123456.789,
  "commit_deadline": 1707123756.789,
  "reveal_deadline": 1707124056.789,
  "results": {
    "yes": 3,
    "no": 0,
    "total": 3,
    "quorum_met": true,
    "outcome": "APPROVED"
  },
  "audit_trail": {...}
}
```

## Jury Composition

### Permanent Off-Chain Auditors
- Human auditors or trusted agents
- Always available for jury duty
- Review all major task proposals

### Rotating On-Chain Agents
- Agents selected from on-chain pool each epoch
- Operate from read-only state snapshot
- Cannot write to on-chain state during jury duty
- Returned to normal duty after audit

### Section Diversity
The jury enforces section diversity to prevent monoculture:
- Max 75% of jury from any single section
- Sections: governance, security, economics, operations
- Example: 5-member jury with (2, 1, 1, 1) distribution across sections

## Audit Logging

### Event Types
- `SESSION_CREATED`: Jury session initialized
- `COMMIT_SUBMITTED`: Vote commitment recorded
- `REVEAL_PHASE_ADVANCED`: Transitioned to reveal phase
- `VOTE_REVEALED`: Vote revealed and verified
- `VOTES_AGGREGATED`: Results tallied
- `SESSION_COMPLETED`: Audit log sealed
- `AUDIT_RETRIEVAL`: Log accessed

### Integrity Verification

Each audit log is sealed with a SHA256 hash:

```python
from swarm.jury import AuditLogger

logger = AuditLogger()
log = logger.create_log("session-id")

# ... record events ...

# Seal log and get hash for on-chain anchor
content_hash = logger.complete_session("session-id")
print(f"Anchor this hash on-chain: {content_hash}")

# Later, verify integrity
is_valid = logger.verify_log_integrity("session-id")
print(f"Log integrity verified: {is_valid}")
```

### Accessing Audit Trails

```python
# Retrieve full audit trail
trail_json = logger.get_audit_trail("session-id")

# Get statistics
stats = logger.get_log_stats("session-id")
print(f"Events: {stats['event_count']}")
print(f"Types: {stats['event_types']}")
print(f"Hash: {stats['content_hash']}")
print(f"On-chain anchor: {stats['on_chain_anchor']}")
```

## Quorum Rules

- **Minimum jury size**: 3 members
- **Approval threshold**: 66% majority (2/3)
- **Formula**: `yes_count / jury_size >= 0.66 AND jury_size >= 3`

### Examples
- 3 members: need 2 yes votes to pass (66.7%)
- 5 members: need 4 yes votes to pass (80%)
- 6 members: need 4 yes votes to pass (66.7%)

## Severity Classification

Tasks are classified before jury routing:

### MAJOR (→ Jury Review)
- Governance rule changes
- Treasury transactions > 100k X3
- Agent role modifications
- Security boundary changes
- Schema migrations
- on-chain state modifications

### MINOR (→ Core Approval Only)
- Configuration updates
- Monitoring/telemetry changes
- Documentation updates
- Routine operations
- Log management
- Non-breaking dependency updates

## Integration Points

### On-Chain Anchoring
When a jury session completes:
1. Audit log hash is computed
2. Hash is sent to blockchain validator
3. Validator posts hash in transaction
4. Transaction hash stored in audit log

Production implementation requires blockchain integration layer.

### Task Intent Files
Jury tasks are defined in `.md` files with YAML front matter:

```yaml
---
id: task-2024-001
type: law
severity: major
proposer: alice@example.com
section: governance
created_at: 2024-02-08T10:00:00Z
hash: 0x1234abcd...
---

# Change: Update Inflation Rate

## Proposal
Increase inflation rate from 2% to 2.5% starting epoch 42.

## Rationale
Network analysis shows suboptimal rewards distribution.

## Impact
- 0.5% increase in token supply per epoch
- Affects all stake holders
- Requires governance approval
```

## Testing

### Running Tests
```bash
# Jury manager tests
pytest swarm/tests/test_jury.py -v

# Audit logging tests
pytest swarm/tests/test_jury_audit.py -v

# API endpoint tests
pytest swarm/tests/test_jury_api.py -v
```

## Next Steps

1. **On-Chain Integration** (Phase 3)
   - Add blockchain adapter for anchor recording
   - Implement transaction hooks
   - Validate hash anchors

2. **Encryption** (Phase 3)
   - Add AES encryption for detailed logs
   - Implement key rotation
   - Add HSM support for production

3. **Access Control** (Phase 3)
   - Implement role-based audit trail access
   - Add time-based restrictions
   - Create audit log retrieval audit events

4. **Persistence** (Phase 4)
   - Migrate from in-memory to persistent store
   - Add database backend
   - Implement log archival and retention

## References

- [Jury Specification](../../specs/swarm/spec.md)
- [Orchestra Governance Spec](../../specs/orchestra-governance/spec.md)
- [Severity Taxonomy](../../specs/orchestra-governance/severity-taxonomy.md)
- [Proposal](../proposal.md)
- [Design](../design.md)
