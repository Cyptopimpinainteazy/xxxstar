# EPIC-1 L0-L1 Foundation — Execution Master Summary

**Phase**: Month 1 of X3 Execution Roadmap  
**Tasks**: L0-01, L0-02, L0-03, L1-01 (4 critical foundation tasks)  
**Timeline**: 4-5 days total execution (1-2 + 1 + 1-2 + 1 days)  
**Blocking**: All EPIC-2 through EPIC-9 work  

---

## Quick Reference: Four Critical Tasks

| Task | Name | Duration | Prerequisite | What It Does |
|------|------|----------|--------------|-------------|
| **L0-01** | Genesis Configuration Engine | 1-2 days | (none) | Immutable chain configuration, validation, migrations |
| **L0-02** | Change Ledger | 1 day | L0-01 | Append-only audit trail, hash chaining, queryable |
| **L0-03** | Capability Registry | 1-2 days | L0-01, L0-02 | Type-safe permissions, dependency resolution |
| **L1-01** | Operator Registry | 1 day | L0-01, L0-02, L0-03 | Validator management, lifecycle, capability validation |

---

## Detailed Specification Files

All production-ready specifications created:

- **[docs/reports/L0-01-GENESIS-CONFIGURATION-ENGINE.md](./docs/reports/L0-01-GENESIS-CONFIGURATION-ENGINE.md)**
  - 450+ lines of complete specification
  - Type definitions, validation rules, schema structure
  - Testing specifications, acceptance criteria

- **[docs/reports/L0-02-APPEND-ONLY-CHANGE-LEDGER.md](./docs/reports/L0-02-APPEND-ONLY-CHANGE-LEDGER.md)**
  - 350+ lines of complete specification
  - Writer/Reader interface, SQLite storage, query operations
  - Hash chain validation, concurrent safety

- **[docs/reports/L0-03-CAPABILITY-REGISTRY.md](./docs/reports/L0-03-CAPABILITY-REGISTRY.md)**
  - 400+ lines of complete specification
  - Registry with CRUD, dependency resolution, validation
  - Circular dependency detection, topological sort

- **[docs/reports/L1-01-OPERATOR-REGISTRY.md](./docs/reports/L1-01-OPERATOR-REGISTRY.md)**
  - 450+ lines of complete specification
  - Registry with capability validation, lifecycle management
  - Integration with L0-01/02/03, end-to-end testing

---

## Execution Sequence (Strict Order)

### Day 1-2: L0-01 Genesis Configuration Engine

**What to Build**:
1. `internal/genesis/types.go` — Genesis, Validator, Account, ConsensusParams types
2. `internal/genesis/validator.go` — SchemaValidator with complete constraint checking
3. `internal/genesis/migration.go` — MigrationRegistry with v1.0.0 → v1.1.0 example
4. `internal/genesis/storage.go` — FileStorage with atomic writes
5. `*_test.go` — 95%+ coverage tests

**Key Requirements**:
- ✓ Genesis type with Version, ChainID, Validators, Accounts, AppState
- ✓ Validation: semver versions, hex addresses, unique validators, power > 0
- ✓ ComputeHash() returns deterministic SHA256
- ✓ FileStorage atomic writes (write-to-temp, atomic rename)
- ✓ MigrationRegistry for schema version transitions
- ✓ All tests passing with `-race` detector

**Acceptance Criteria**:
- [ ] All required types defined and validated
- [ ] 95%+ test coverage
- [ ] All tests pass with race detector
- [ ] Complete Go doc comments
- [ ] No lint warnings

**Unblocks**: L0-02, L0-03, L0-04, L0-05

---

### Day 2: L0-02 Append-Only Change Ledger

**What to Build**:
1. `internal/ledger/types.go` — ChangeEntry, ChangeOperation, QueryFilter, AuditTrail
2. `internal/ledger/writer.go` — AppendOnlyWriter with hash chaining
3. `internal/ledger/reader.go` — AppendOnlyReader with queryable interface
4. `internal/ledger/storage.go` — SQLiteStorage with WAL mode, indexes
5. `*_test.go` — Concurrent write safety, hash chain validation

**Key Requirements**:
- ✓ ChangeEntry with ID, Timestamp, Operation, Hash, Chain
- ✓ AppendOnlyWriter: atomic writes, auto ID increment, hash computation, chain linking
- ✓ Reader: QueryByID, QueryByFilter, QueryAuditTrail, QueryTimeRange, QueryEntityType
- ✓ SQLiteStorage: WAL mode, ACID guarantees, indexes on entity/timestamp/operation
- ✓ Hash chain validation preventing corruption
- ✓ Concurrent write safety (tested with -race)

**Acceptance Criteria**:
- [ ] Write/Read interfaces working correctly
- [ ] Hash chain validated on all queries
- [ ] Concurrent write safety verified
- [ ] Performance: >1000 writes/sec, <100ms queries
- [ ] 95%+ test coverage
- [ ] All tests passing with race detector

**Unblocks**: L0-03, L0-04, all upper EPIC work

---

### Day 2-3: L0-03 Capability Registry (Parallel with L0-02 completion)

**What to Build**:
1. `internal/capability/types.go` — Capability, CapabilityRef, CapabilityStatus
2. `internal/capability/registry.go` — InMemoryRegistry with full CRUD
3. `internal/capability/validator.go` — Circular dependency detection, version constraints
4. `internal/capability/resolver.go` — DependencyGraph, topological sort
5. `*_test.go` — Dependency resolution, circular detection, permission lookup

**Key Requirements**:
- ✓ Capability type with ID, Version, Permissions, Dependencies, Status
- ✓ Registry: RegisterCapability, FindByID, FindByVersion, FindByPermission
- ✓ Circular dependency detection with clear error messages
- ✓ DependencyGraph resolution with topological ordering
- ✓ Version constraint validation (semver ranges)
- ✓ Status lifecycle (ACTIVE, DEPRECATED, REMOVED)

**Acceptance Criteria**:
- [ ] All registry operations working
- [ ] Circular dependencies detected and reported
- [ ] Dependency graph construction verified
- [ ] Permission lookup efficient
- [ ] 95%+ test coverage
- [ ] All tests passing with race detector

**Unblocks**: L1-01, L1-02

---

### Day 4: L1-01 Operator Registry

**What to Build**:
1. `internal/operator/types.go` — Operator, OperatorStatus, StatusTransition
2. `internal/operator/registry.go` — InMemoryRegistry with capability validation
3. `internal/operator/validator.go` — Capability enforcement, state machine
4. `internal/operator/lifecycle.go` — State transitions (ACTIVE ↔ INACTIVE → SUSPENDED)
5. `*_test.go` — Registration, capability validation, state transitions, ledger integration

**Key Requirements**:
- ✓ Operator type with ID, Address, Capabilities, Status, Metadata
- ✓ Registry: RegisterOperator, FindByID, FindByAddress, ListByCapability
- ✓ Capability validation against L0-03 registry before registration
- ✓ Lifecycle state machine: ACTIVE ↔ INACTIVE, SUSPENDED, REMOVED transitions
- ✓ Audit trail integration with L0-02 Change Ledger
- ✓ Address uniqueness enforcement
- ✓ Concurrent safety

**Acceptance Criteria**:
- [ ] Operator registration with capability validation
- [ ] State transitions enforce valid paths
- [ ] Ledger entries created for all mutations
- [ ] FindByAddress works correctly
- [ ] ListByCapability returns correct set
- [ ] 95%+ test coverage
- [ ] All tests passing with race detector
- [ ] End-to-end integration test passing

**Unblocks**: L1-02, L1-03, L1-04, L1-05 (all higher L1 work)

---

## Integration Checkpoints

### After L0-01 Complete
```
✓ Genesis is defined, validated, stored
✗ Not yet persisted to ledger
→ PROCEED to L0-02
```

### After L0-02 Complete
```
✓ Ledger is working, queryable, hash-chained
✓ Genesis mutations can be recorded
✗ No capabilities defined yet
→ PROCEED to L0-03
```

### After L0-03 Complete
```
✓ Capabilities are registered and discovered
✓ Dependencies resolved
✗ No operators yet
→ PROCEED to L1-01
```

### After L1-01 Complete
```
✓ Operators registered with capability validation
✓ Audit trail complete for all mutations
✓ State machine working
✓ L0 FOUNDATION COMPLETE ✓

→ UNBLOCKS EPIC-2 (Formal Verification Phase)
```

---

## Quality Gates Checklist

**All Four Tasks Must Pass**:

- [ ] `go test ./internal/genesis/...` — all passing
- [ ] `go test ./internal/ledger/...` — all passing
- [ ] `go test ./internal/capability/...` — all passing
- [ ] `go test ./internal/operator/...` — all passing

**All With Race Detector**:
- [ ] `go test -race ./internal/genesis/...` — no races
- [ ] `go test -race ./internal/ledger/...` — no races
- [ ] `go test -race ./internal/capability/...` — no races
- [ ] `go test -race ./internal/operator/...` — no races

**Coverage Requirements**:
- [ ] `go tool cover` shows >95% for all four modules

**Linting**:
- [ ] `golangci-lint run ./internal/genesis/...` — zero warnings
- [ ] `golangci-lint run ./internal/ledger/...` — zero warnings
- [ ] `golangci-lint run ./internal/capability/...` — zero warnings
- [ ] `golangci-lint run ./internal/operator/...` — zero warnings

**Documentation**:
- [ ] `go doc ./internal/genesis/...` — all exported symbols documented
- [ ] `go doc ./internal/ledger/...` — all exported symbols documented
- [ ] `go doc ./internal/capability/...` — all exported symbols documented
- [ ] `go doc ./internal/operator/...` — all exported symbols documented

**Integration Test**:
- [ ] End-to-end test in `tests/integration/foundation_test.go`
  - Load Genesis (L0-01) ✓
  - Create Ledger (L0-02) ✓
  - Register Capabilities (L0-03) ✓
  - Register Operators (L1-01) ✓
  - Query audit trail ✓
  - Validate hash chains ✓

---

## File Structure

After completion, directory layout:

```
x3/
├── cmd/
│   └── x3-genesis/
│       └── main.go
├── internal/
│   ├── genesis/
│   │   ├── types.go
│   │   ├── validator.go
│   │   ├── migration.go
│   │   ├── storage.go
│   │   ├── types_test.go
│   │   ├── validator_test.go
│   │   ├── migration_test.go
│   │   └── storage_test.go
│   ├── ledger/
│   │   ├── types.go
│   │   ├── writer.go
│   │   ├── reader.go
│   │   ├── storage.go
│   │   ├── errors.go
│   │   ├── writer_test.go
│   │   ├── reader_test.go
│   │   └── storage_test.go
│   ├── capability/
│   │   ├── types.go
│   │   ├── registry.go
│   │   ├── validator.go
│   │   ├── resolver.go
│   │   ├── errors.go
│   │   ├── registry_test.go
│   │   ├── validator_test.go
│   │   └── resolver_test.go
│   └── operator/
│       ├── types.go
│       ├── registry.go
│       ├── validator.go
│       ├── lifecycle.go
│       ├── errors.go
│       ├── registry_test.go
│       ├── validator_test.go
│       ├── lifecycle_test.go
│       └── integration_test.go
├── tests/
│   └── integration/
│       └── foundation_test.go
├── go.mod
├── go.sum
├── Makefile
└── docs/root/README.md
```

---

## Makefile Targets (Create These)

```makefile
# Test all
test:
	go test ./internal/...

# Test with race detector
test-race:
	go test -race ./internal/...

# Coverage
coverage:
	go test -coverprofile=coverage.out ./internal/...
	go tool cover -html=coverage.out

# Lint
lint:
	golangci-lint run ./internal/...

# Build CLI
build:
	go build -o x3-genesis ./cmd/x3-genesis

# Run integration test
integration-test:
	go test -v ./tests/integration/...

# All quality gates
quality: test-race lint coverage
	@echo "✓ All quality gates passed"
```

---

## Development Notes

### Dependency Assumptions

All code should use only standard library + minimal deps:
```go
import (
    "crypto/sha256"
    "database/sql"
    "encoding/json"
    "sync"
    "time"
    // Standard lib only
    _ "github.com/mattn/go-sqlite3"  // SQLite driver
)
```

### Error Handling Pattern

```go
// Always return named error types, not string errors
return nil, NewCapabilityError("find", fmt.Sprintf("capability %s not found", id))

// Errors include context for debugging
type CapabilityError struct {
    Code    string      // Error category
    Context string      // Operation context
    Message string      // User-friendly message
}
```

### Logging Integration

When completed, each module will add structured logging (context propagation):
```go
logger.InfoContext(ctx, "operation_started",
  slog.String("operation", "RegisterCapability"),
  slog.String("capability_id", cap.ID))
```

This will be integrated after L0-01 foundation is complete (EPIC-2 phase).

---

## Success Criteria: L0-01 EPIC Complete

You'll know EPIC-1 (Core Protocol) Phase 1 is complete when:

✅ **L0-01**: Genesis configuration loaded, validated, stored  
✅ **L0-02**: All genesis changes recorded in immutable audit trail  
✅ **L0-03**: Capabilities registered and dependencies resolved  
✅ **L1-01**: Operators registered with capability validation  
✅ **Integration**: End-to-end flow working: Genesis → Ledger → Capabilities → Operators  
✅ **Quality**: All tests passing, 95%+ coverage, zero race conditions  
✅ **Documentation**: Complete Go docs, examples, ADRs  

---

## What Happens Next

Once L0-01 through L1-01 complete:

**Week 2-3** (EPIC-2 Formal Verification):
- TLA+ specs for kernel invariants
- Coq proofs for critical properties
- Automated property testing

**Week 4** (EPIC-1 continued - L1-02 through L1-05):
- Bonding engine
- Slashing engine
- Governance
- Attack simulator

**Week 5+** (EPIC-3, EPIC-6):
- Command Center UI development
- Economic modeling simulations

---

## Execution Mindset

- **Deterministic**: All hashes are reproducible
- **Immutable**: Changes are recorded, never modified
- **Auditable**: Complete audit trail for compliance
- **Safe**: Race detection, linting, coverage validation
- **Testable**: 95%+ coverage before proceeding
- **Documented**: Go docs, examples, architecture decisions

**Start with L0-01. Ship it complete. Move to L0-02. Repeat.** 🚀

Each task is self-contained but integrates cleanly with the next. Do not skip testing or jump ahead. The foundation must be solid.

---

## Questions Before Execution?

Refer to specification files for details:

- **L0-01 Details**: [docs/reports/L0-01-GENESIS-CONFIGURATION-ENGINE.md](./docs/reports/L0-01-GENESIS-CONFIGURATION-ENGINE.md)
- **L0-02 Details**: [docs/reports/L0-02-APPEND-ONLY-CHANGE-LEDGER.md](./docs/reports/L0-02-APPEND-ONLY-CHANGE-LEDGER.md)
- **L0-03 Details**: [docs/reports/L0-03-CAPABILITY-REGISTRY.md](./docs/reports/L0-03-CAPABILITY-REGISTRY.md)
- **L1-01 Details**: [docs/reports/L1-01-OPERATOR-REGISTRY.md](./docs/reports/L1-01-OPERATOR-REGISTRY.md)

**All code examples are production-ready. All type definitions are complete. All test specifications are detailed.**

**Execute now.** 🎯
