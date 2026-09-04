# EPIC-1 Foundation — Daily Execution Checklist

**Start Date**: [TODAY]  
**Target Completion**: Day 4-5  
**Success Condition**: All 4 tasks complete, all tests passing, all quality gates green

---

## DAY 1: L0-01 Genesis Configuration Engine (Part 1)

### Morning: Types & Validation (4-5 hours)

**Files to Create**:
- [ ] `internal/genesis/types.go` (Genesis, Validator, Account, ConsensusParams)
- [ ] `internal/genesis/validator.go` (SchemaValidator with constraints)
- [ ] `internal/genesis/errors.go` (ValidationError)

**Acceptance per File**:
- [ ] types.go: All 4 types with JSON tags, validation tags, ComputeHash() method
- [ ] validator.go: SchemaValidator created, ValidateGenesisSchema() working
- [ ] errors.go: ValidationError with Field, Value, Constraint, Message fields

**Verify Before Moving On**:
```bash
go test ./internal/genesis/validator_test.go   # Just schema tests
go fmt ./internal/genesis/...
go vet ./internal/genesis/...
```

### Afternoon: Storage & Migration (3-4 hours)

**Files to Create**:
- [ ] `internal/genesis/storage.go` (FileStorage interface & impl)
- [ ] `internal/genesis/migration.go` (MigrationRegistry with example)

**Acceptance per File**:
- [ ] storage.go: Save() atomic writes, Load() reads, Version() queries
- [ ] migration.go: Migrate() function, v1.0.0→v1.1.0 example migration

**Verify Before EOD**:
```bash
go test ./internal/genesis/storage_test.go      # File I/O tests
go test ./internal/genesis/migration_test.go    # Migration tests
```

### Evening: Test Suite (2-3 hours)

**Files to Create**:
- [ ] `internal/genesis/types_test.go` (type/hash tests)
- [ ] `internal/genesis/validator_test.go` (validation edge cases)
- [ ] `internal/genesis/storage_test.go` (file I/O)
- [ ] `internal/genesis/migration_test.go` (version transitions)

**Coverage Target**: >90% by end of day (will get to 95% on Day 2)

**EOD Check**:
```bash
go test ./internal/genesis/... -v
go test -race ./internal/genesis/...          # Should pass
go tool cover -html=coverage.out               # View coverage
```

**Blockers?** ⏸️
- If hash determinism failing → debug ComputeHash() normalization
- If storage atomicity issues → check temp file cleanup on failure
- If any test hanging → likely synchronization issue (rare at this stage)

---

## DAY 2: L0-01 Genesis Configuration Engine (Part 2) + Start L0-02

### Morning: L0-01 Completion (3-4 hours)

**Tasks**:
- [ ] Fix any failing tests from Day 1
- [ ] Add missing edge case tests to reach 95% coverage
- [ ] Add Go doc comments to all exported types/functions
- [ ] Run all quality gates

**Completion Check**:
```bash
go test -race ./internal/genesis/... -v        # All passing?
go tool cover                                  # >95% coverage?
golangci-lint run ./internal/genesis/...       # Zero warnings?
go doc ./internal/genesis/Genesis              # Docs present?
```

**L0-01 Definition of Done**:
- [ ] All tests passing
- [ ] All tests passing with -race
- [ ] >95% coverage
- [ ] Zero lint warnings
- [ ] Complete Go docs
- [ ] Commit to feature branch

### Afternoon: L0-02 Types & Writer (4-5 hours)

**Files to Create**:
- [ ] `internal/ledger/types.go` (ChangeEntry, ChangeOperation, QueryFilter)
- [ ] `internal/ledger/writer.go` (AppendOnlyWriter with chaining)
- [ ] `internal/ledger/errors.go` (LedgerError)

**Acceptance per File**:
- [ ] types.go: ChangeEntry with all fields, ComputeHash(), hash chain support
- [ ] writer.go: Append() with ID auto-increment, hash computation, chain linking
- [ ] errors.go: LedgerError with Code, Field, Message

**Verify**:
```bash
go test ./internal/ledger/writer_test.go -v    # Basic append tests
go fmt ./internal/ledger/...
```

### Evening: L0-02 Reader & Storage Foundation (2-3 hours)

**Files to Start**:
- [ ] `internal/ledger/reader.go` (AppendOnlyReader skeleton)
- [ ] `internal/ledger/storage.go` (Storage interface, SQLiteStorage stub)

**Don't Worry About**: Full SQLite implementation yet, just interfaces

---

## DAY 3: L0-02 + L0-03 (Parallel Track)

### L0-02 Completion (Morning: 3-4 hours)

**Files Finish**:
- [ ] Complete `internal/ledger/reader.go` (QueryByID, QueryByFilter, QueryAuditTrail)
- [ ] Complete `internal/ledger/storage.go` (full SQLiteStorage impl)
- [ ] Complete test suite (writer, reader, storage, integration)

**Key Tests**:
- [ ] Concurrent writes (use 'go test -race')
- [ ] Hash chain validation on queries
- [ ] Query performance (<100ms for 10k entries)

**L0-02 Definition of Done**:
- [ ] Writer creates entries with hash chains
- [ ] Reader queries work (ByID, ByFilter, ByTimeRange, AuditTrail)
- [ ] SQLiteStorage persists to disk
- [ ] Hash chain validates correctly
- [ ] Tests passing with -race
- [ ] >95% coverage
- [ ] Commit

### L0-03 Start (Afternoon: 4-5 hours)

**Files to Create**:
- [ ] `internal/capability/types.go` (Capability, CapabilityRef, CapabilityStatus)
- [ ] `internal/capability/registry.go` (InMemoryRegistry CRUD)
- [ ] `internal/capability/validator.go` (CapabilityValidator)

**Acceptance**:
- [ ] Registry: RegisterCapability, FindByID, FindByVersion, FindByPermission, ListAll
- [ ] Validator: Validates ID format, version (semver), permissions, status
- [ ] Basic tests for registration and lookup

**Verify**:
```bash
go test ./internal/capability/registry_test.go -v
```

---

## DAY 4: L0-03 + L1-01

### L0-03 Completion (Morning: 3-4 hours)

**Files Finish**:
- [ ] Add DependencyGraph resolution (topological sort)
- [ ] Circular dependency detection
- [ ] GetStats()
- [ ] Complete test suite (all edge cases)

**Key Tests**:
- [ ] Circular dependency detection
- [ ] Dependency graph construction
- [ ] Permission lookup
- [ ] Status transitions (ACTIVE → DEPRECATED)

**L0-03 Definition of Done**:
- [ ] All registry operations working
- [ ] Circular dependencies detected
- [ ] Dependency graph topologically sorted
- [ ] 95%+ coverage
- [ ] All tests with -race pass
- [ ] Commit

### L1-01 Start (Afternoon: 4-5 hours)

**Files to Create**:
- [ ] `internal/operator/types.go` (Operator, OperatorStatus, StatusTransition)
- [ ] `internal/operator/registry.go` (InMemoryRegistry with L0-03 validation)
- [ ] `internal/operator/validator.go` (capability validation)
- [ ] `internal/operator/lifecycle.go` (state machine)

**Integration with L0-03**:
- [ ] Register() validates all capabilities exist in registry
- [ ] Block registration if capability missing/inactive
- [ ] Clear error messages when validation fails

**Basic Tests**:
- [ ] Register operator (with capability validation)
- [ ] Find by ID/Address
- [ ] State transitions (ACTIVE → INACTIVE → ACTIVE)

**Verify**:
```bash
go test ./internal/operator/registry_test.go::TestRegisterOperator -v
go test -race ./internal/operator/... -v
```

---

## DAY 5: L1-01 Completion + Integration

### Morning: L1-01 Finish (4-5 hours)

**Complete Files**:
- [ ] Finish `internal/operator/registry.go` (all methods)
- [ ] Finish `internal/operator/lifecycle.go` (all transitions)
- [ ] Complete test suite (all edge cases, concurrent writes)

**Integration with L0-02**:
- [ ] RegisterOperator creates ChangeEntry in ledger
- [ ] UpdateOperator records previous/new state in ledger
- [ ] DeactivateOperator records in ledger

**Key Tests**:
- [ ] Registration with missing capability → error
- [ ] State transition validation
- [ ] Ledger entries created
- [ ] FindByAddress uniqueness
- [ ] ListByCapability returns correct set

**L1-01 Definition of Done**:
- [ ] All registry operations working
- [ ] Capability validation enforced
- [ ] State machine working
- [ ] Ledger integration working
- [ ] 95%+ coverage
- [ ] All tests with -race pass
- [ ] Commit

### Afternoon: Integration Testing (3-4 hours)

**Files to Create**:
- [ ] `tests/integration/foundation_test.go`

**Test Flow**:
```
1. Create Genesis (L0-01)
2. Load Genesis (L0-01)
3. Create ChangeEntry for genesis creation (L0-02)
4. Register Capability (L0-03) → Create in ledger (L0-02)
5. Register Operator (L1-01) → Validate against L0-03 → Create in ledger (L0-02)
6. Query AuditTrail for operator
7. Validate hash chains throughout
```

**Integration Test Checklist**:
- [ ] Genesis load succeeds
- [ ] Ledger accepts entries
- [ ] Capability registration succeeds
- [ ] Operator registration with invalid capability → fails
- [ ] Operator registration with valid capability → succeeds
- [ ] Query audit trail returns correct entries
- [ ] Hash chains validate correctly
- [ ] All assertions pass

---

## Quality Gates → RUN BEFORE MERGING

### Pre-Merge Checklist

```bash
# All tests passing
go test ./internal/genesis/... -v         # ✓
go test ./internal/ledger/... -v          # ✓
go test ./internal/capability/... -v      # ✓
go test ./internal/operator/... -v        # ✓
go test ./tests/integration/... -v        # ✓

# No race conditions
go test -race ./internal/...              # ✓
go test -race ./tests/integration/...     # ✓

# Coverage
go test -coverprofile=coverage.out ./internal/...
go tool cover -func=coverage.out | grep total  # Shows % (must be >95%)

# No lint warnings
golangci-lint run ./internal/...          # ✓ (zero warnings)

# Documentation
go doc ./internal/genesis/Genesis         # ✓
go doc ./internal/ledger/Writer           # ✓
go doc ./internal/capability/Registry     # ✓
go doc ./internal/operator/Registry       # ✓

# Build commands
go build ./internal/genesis/...           # ✓
go build ./internal/ledger/...            # ✓
go build ./internal/capability/...        # ✓
go build ./internal/operator/...          # ✓
```

### If Any Check ❌ Fails

- [ ] Don't proceed to next task
- [ ] Debug the failure immediately
- [ ] Add test cases for the edge case
- [ ] Run quality gates again
- [ ] Only proceed when ✓ all green

---

## Blockers & Escalation

**If stuck on**:

| Problem | Likely Cause | Fix |
|---------|--------------|-----|
| Test hanging | Deadlock in mutex | Check for nested locks |
| Race detector failures | Concurrent map access | Use RWMutex, protect with locks |
| Hash not deterministic | JSON marshaling order | Use json.Marshal (sorts keys) or manual ordering |
| Coverage <95% | Not testing error paths | Add negative tests (invalid input, missing fields) |
| Lint warnings | Code style issues | Run `gofmt -w .`, then check golangci-lint output |
| Integration test fails | Previous modules broken | Rerun tests for L0-01/02/03 first |

---

## Success Verification (End of Day 5)

### ✅ All Four Modules Complete

- L0-01: Genesis Configuration Engine
- L0-02: Append-Only Change Ledger
- L0-03: Capability Registry
- L1-01: Operator Registry

### ✅ All Quality Metrics Met

- >95% coverage across all modules
- All tests passing
- All tests passing with -race detector
- Zero lint warnings
- Complete Go documentation
- Integration test passing

### ✅ All Tasks Committed

Push feature branch with:
- All 4 modules complete
- All tests passing
- All documentation present
- Sample usage examples
- Architecture decision records (why SQLite, why in-memory registry, etc.)

### ✅ Ready for EPIC-2

Next phase (Formal Verification) can now begin:
- TLA+ specs for Genesis
- Coq proofs for ledger properties
- Property-based testing

---

## Daily Standup Template

Use this each morning:

**Yesterday Completed**:
- [ ] Task X: [files completed], [tests passing]

**Today Planning**:
- [ ] Task Y: [files to create], [acceptance criteria]

**Blockers**:
- [ ] None / [specific issue]

**Confidence Level**: [Low/Medium/High]

---

## Success Tracking

Track completed items in this document as you go:

**L0-01**: [████████░░] 90% complete
**L0-02**: [████░░░░░░] 45% complete
**L0-03**: [███░░░░░░░] 30% complete
**L1-01**: [░░░░░░░░░░] 0% complete

**End Goal**: All four bars at 100% ✅

---

## Remember

1. **Quality over speed** - 95%+ coverage is non-negotiable
2. **One task at a time** - Don't jump ahead
3. **Tests drive design** - Write tests as you code
4. **Determinism matters** - All hashes must be reproducible
5. **Concurrency from day 1** - Race testing everything

**You got this.** Knock out L0-01 today. Ship it solid. 🚀
