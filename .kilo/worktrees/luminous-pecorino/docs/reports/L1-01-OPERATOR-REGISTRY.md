# L1-01: Operator Registry — Production Specification

**Task ID**: L1-01  
**Epic**: EPIC-1 (Core Protocol & Kernel)  
**Status**: Ready for Implementation  
**Estimated Duration**: 1 day  
**Prerequisites**: L0-01, L0-02, L0-03 (Genesis, Ledger, Capability Registry)  
**Unblocks**: L1-02 (Bonding), L1-03 (Slashing), higher L1 work  
**Related Proposal**: X3-PROPOSAL-001

---

## Executive Summary

The Operator Registry manages validators/operators in X3. It:
- Registers operators with required capabilities validated against registry
- Manages operator lifecycle (active/inactive/suspended states)
- Enforces capability requirements before registration
- Records all mutations via L0-02 Change Ledger audit trail
- Provides operator discovery and validation queries

**Design Principle**: Operators are mutable; state changes are immutable (recorded in ledger).

---

## Domain Types

### Operator Entity Model

```go
// internal/operator/types.go

package operator

import (
    "crypto/sha256"
    "encoding/hex"
    "encoding/json"
    "fmt"
    "time"
)

// Operator represents a validator/participant in X3.
type Operator struct {
    // ID is unique operator identifier
    ID string `json:"id" validate:"required,alphanum,min=3,max=128"`
    
    // Name is human-readable name
    Name string `json:"name" validate:"required,min=1,max=256"`
    
    // Address is operator's blockchain address (from genesis)
    Address string `json:"address" validate:"required,hexadecimal,len=40"`
    
    // Capabilities are IDs of capabilities this operator has
    Capabilities []string `json:"capabilities" validate:"required,min=1,dive"`
    
    // Status indicates current lifecycle state
    Status OperatorStatus `json:"status" validate:"required"`
    
    // Metadata is arbitrary key-value data (endpoints, version, etc.)
    Metadata map[string]string `json:"metadata,omitempty"`
    
    // CreatedAt is registration timestamp
    CreatedAt time.Time `json:"createdAt" validate:"required"`
    
    // LastUpdatedAt is last state change
    LastUpdatedAt time.Time `json:"lastUpdatedAt" validate:"required"`
    
    // DeactivatedAt is when operator was deactivated (if applicable)
    DeactivatedAt *time.Time `json:"deactivatedAt,omitempty"`
    
    // Hash is SHA256 of normalized operator (computed)
    Hash string `json:"hash,omitempty"`
}

// OperatorStatus represents lifecycle state.
type OperatorStatus string

const (
    StatusActive      OperatorStatus = "ACTIVE"
    StatusInactive    OperatorStatus = "INACTIVE"
    StatusSuspended   OperatorStatus = "SUSPENDED"
    StatusRemoved     OperatorStatus = "REMOVED"
)

// OperatorStatusTransition defines valid state transitions.
type OperatorStatusTransition struct {
    From OperatorStatus
    To   OperatorStatus
}

var ValidTransitions = map[OperatorStatusTransition]bool{
    {StatusActive, StatusInactive}:   true,
    {StatusActive, StatusSuspended}:  true,
    {StatusActive, StatusRemoved}:    true,
    {StatusInactive, StatusActive}:   true,
    {StatusInactive, StatusSuspended}: true,
    {StatusInactive, StatusRemoved}:  true,
    {StatusSuspended, StatusActive}:  true,
    {StatusSuspended, StatusInactive}: true,
    {StatusSuspended, StatusRemoved}:  true,
}

// ComputeHash returns SHA256 of normalized operator.
func (o *Operator) ComputeHash() (string, error) {
    copy := *o
    copy.Hash = ""
    
    normalized, err := json.Marshal(copy)
    if err != nil {
        return "", fmt.Errorf("marshal: %w", err)
    }
    
    hash := sha256.Sum256(normalized)
    return hex.EncodeToString(hash[:]), nil
}

// OperatorRegistryStats contains registry statistics.
type OperatorRegistryStats struct {
    // TotalOperators is total count
    TotalOperators int `json:"totalOperators"`
    
    // ByStatus maps status to count
    ByStatus map[string]int `json:"byStatus"`
    
    // ByCapability maps capability ID to operator count
    ByCapability map[string]int `json:"byCapability"`
    
    // AverageCapabilities is mean capability count per operator
    AverageCapabilities float64 `json:"averageCapabilities"`
}

// OperatorValidationResult contains validation outcome.
type OperatorValidationResult struct {
    // IsValid indicates if operator passed validation
    IsValid bool `json:"isValid"`
    
    // Errors are validation failures
    Errors []string `json:"errors,omitempty"`
    
    // MissingCapabilities are required but missing
    MissingCapabilities []string `json:"missingCapabilities,omitempty"`
    
    // InvalidCapabilities are found but not in registry
    InvalidCapabilities []string `json:"invalidCapabilities,omitempty"`
}
```

---

## Registry Implementation

### Operator Registry with Capability Validation

```go
// internal/operator/registry.go

package operator

import (
    "fmt"
    "sync"
    "time"
    
    "internal/capability"
    "internal/ledger"
)

// Registry manages operators and their lifecycle.
type Registry interface {
    // RegisterOperator adds new operator
    RegisterOperator(op *Operator) (*ledger.ChangeEntry, error)
    
    // FindByID returns operator by ID
    FindByID(id string) (*Operator, error)
    
    // FindByAddress returns operator by address
    FindByAddress(address string) (*Operator, error)
    
    // ListByCapability returns all operators with capability
    ListByCapability(capID string) ([]*Operator, error)
    
    // ListByStatus returns operators with given status
    ListByStatus(status OperatorStatus) ([]*Operator, error)
    
    // UpdateOperator modifies operator state
    UpdateOperator(op *Operator) (*ledger.ChangeEntry, error)
    
    // DeactivateOperator transitions to INACTIVE
    DeactivateOperator(id string, reason string) (*ledger.ChangeEntry, error)
    
    // ReactivateOperator transitions to ACTIVE
    ReactivateOperator(id string) (*ledger.ChangeEntry, error)
    
    // SuspendOperator transitions to SUSPENDED
    SuspendOperator(id string, reason string) (*ledger.ChangeEntry, error)
    
    // ValidateOperator checks capability requirements
    ValidateOperator(op *Operator) *OperatorValidationResult
    
    // GetStats returns registry statistics
    GetStats() *OperatorRegistryStats
}

// InMemoryRegistry implements Registry with in-memory storage.
type InMemoryRegistry struct {
    mu              sync.RWMutex
    operators       map[string]*Operator              // id -> operator
    addressIndex    map[string]string                 // address -> operator id
    capabilityIndex map[string][]string               // cap id -> [operator ids]
    validator       *OperatorValidator
    ledgerWriter    ledger.Writer
    capRegistry     capability.Registry
}

// NewInMemoryRegistry creates new registry.
func NewInMemoryRegistry(ledgerWriter ledger.Writer, capRegistry capability.Registry) *InMemoryRegistry {
    return &InMemoryRegistry{
        operators:       make(map[string]*Operator),
        addressIndex:    make(map[string]string),
        capabilityIndex: make(map[string][]string),
        validator:       NewOperatorValidator(capRegistry),
        ledgerWriter:    ledgerWriter,
        capRegistry:     capRegistry,
    }
}

// RegisterOperator validates and registers new operator.
func (r *InMemoryRegistry) RegisterOperator(op *Operator) (*ledger.ChangeEntry, error) {
    if op == nil {
        return nil, NewOperatorError("register", "operator is nil")
    }
    
    // Validate against capability registry
    valResult := r.ValidateOperator(op)
    if !valResult.IsValid {
        return nil, NewOperatorError("validate",
            fmt.Sprintf("validation failed: %v", valResult.Errors))
    }
    
    r.mu.Lock()
    defer r.mu.Unlock()
    
    // Check for duplicate ID
    if _, exists := r.operators[op.ID]; exists {
        return nil, NewOperatorError("register",
            fmt.Sprintf("operator %s already exists", op.ID))
    }
    
    // Check for duplicate address
    if existingID, exists := r.addressIndex[op.Address]; exists {
        return nil, NewOperatorError("register",
            fmt.Sprintf("address already registered by operator %s", existingID))
    }
    
    // Set metadata
    now := time.Now().UTC()
    op.CreatedAt = now
    op.LastUpdatedAt = now
    op.Status = StatusActive
    
    // Compute hash
    hash, err := op.ComputeHash()
    if err != nil {
        return nil, err
    }
    op.Hash = hash
    
    // Store
    r.operators[op.ID] = op
    r.addressIndex[op.Address] = op.ID
    
    // Update capability index
    for _, capID := range op.Capabilities {
        r.capabilityIndex[capID] = append(r.capabilityIndex[capID], op.ID)
    }
    
    // Create ledger entry
    newStateJSON, _ := json.Marshal(op)
    entry := &ledger.ChangeEntry{
        Timestamp:  now,
        Operation:  ledger.OperationCreate,
        EntityType: "operator",
        EntityID:   op.ID,
        NewState:   newStateJSON,
        ActorID:    "genesis",
    }
    
    // Write to ledger
    if err := r.ledgerWriter.Append(entry); err != nil {
        return nil, fmt.Errorf("ledger write failed: %w", err)
    }
    
    return entry, nil
}

// FindByID returns operator by ID.
func (r *InMemoryRegistry) FindByID(id string) (*Operator, error) {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    op, ok := r.operators[id]
    if !ok {
        return nil, NewOperatorError("find",
            fmt.Sprintf("operator %s not found", id))
    }
    
    return op, nil
}

// FindByAddress returns operator by address.
func (r *InMemoryRegistry) FindByAddress(address string) (*Operator, error) {
    r.mu.RLock()
    id, ok := r.addressIndex[address]
    r.mu.RUnlock()
    
    if !ok {
        return nil, NewOperatorError("find",
            fmt.Sprintf("no operator at address %s", address))
    }
    
    return r.FindByID(id)
}

// ListByCapability returns operators with capability.
func (r *InMemoryRegistry) ListByCapability(capID string) ([]*Operator, error) {
    r.mu.RLock()
    ids := r.capabilityIndex[capID]
    r.mu.RUnlock()
    
    var ops []*Operator
    for _, id := range ids {
        if op, err := r.FindByID(id); err == nil {
            ops = append(ops, op)
        }
    }
    
    return ops, nil
}

// ListByStatus returns operators with status.
func (r *InMemoryRegistry) ListByStatus(status OperatorStatus) ([]*Operator, error) {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    var ops []*Operator
    for _, op := range r.operators {
        if op.Status == status {
            ops = append(ops, op)
        }
    }
    
    return ops, nil
}

// UpdateOperator modifies operator state.
func (r *InMemoryRegistry) UpdateOperator(op *Operator) (*ledger.ChangeEntry, error) {
    if op == nil {
        return nil, NewOperatorError("update", "operator is nil")
    }
    
    r.mu.Lock()
    defer r.mu.Unlock()
    
    // Find existing
    existing, ok := r.operators[op.ID]
    if !ok {
        return nil, NewOperatorError("update",
            fmt.Sprintf("operator %s not found", op.ID))
    }
    
    // Save previous state
    prevStateJSON, _ := json.Marshal(existing)
    
    // Update
    op.CreatedAt = existing.CreatedAt
    op.LastUpdatedAt = time.Now().UTC()
    
    hash, _ := op.ComputeHash()
    op.Hash = hash
    
    r.operators[op.ID] = op
    
    // Create ledger entry
    newStateJSON, _ := json.Marshal(op)
    entry := &ledger.ChangeEntry{
        Timestamp:    op.LastUpdatedAt,
        Operation:    ledger.OperationUpdate,
        EntityType:   "operator",
        EntityID:     op.ID,
        PreviousState: prevStateJSON,
        NewState:     newStateJSON,
        ActorID:      "system",
    }
    
    if err := r.ledgerWriter.Append(entry); err != nil {
        return nil, fmt.Errorf("ledger write failed: %w", err)
    }
    
    return entry, nil
}

// DeactivateOperator transitions operator to INACTIVE.
func (r *InMemoryRegistry) DeactivateOperator(id string, reason string) (*ledger.ChangeEntry, error) {
    op, err := r.FindByID(id)
    if err != nil {
        return nil, err
    }
    
    if !ValidTransitions[OperatorStatusTransition{op.Status, StatusInactive}] {
        return nil, NewOperatorError("deactivate",
            fmt.Sprintf("cannot transition from %s to INACTIVE", op.Status))
    }
    
    op.Status = StatusInactive
    op.DeactivatedAt = timePtr(time.Now().UTC())
    if op.Metadata == nil {
        op.Metadata = make(map[string]string)
    }
    op.Metadata["deactivation_reason"] = reason
    
    return r.UpdateOperator(op)
}

// ReactivateOperator transitions operator to ACTIVE.
func (r *InMemoryRegistry) ReactivateOperator(id string) (*ledger.ChangeEntry, error) {
    op, err := r.FindByID(id)
    if err != nil {
        return nil, err
    }
    
    if !ValidTransitions[OperatorStatusTransition{op.Status, StatusActive}] {
        return nil, NewOperatorError("reactivate",
            fmt.Sprintf("cannot transition from %s to ACTIVE", op.Status))
    }
    
    op.Status = StatusActive
    op.DeactivatedAt = nil
    
    return r.UpdateOperator(op)
}

// SuspendOperator transitions operator to SUSPENDED.
func (r *InMemoryRegistry) SuspendOperator(id string, reason string) (*ledger.ChangeEntry, error) {
    op, err := r.FindByID(id)
    if err != nil {
        return nil, err
    }
    
    if !ValidTransitions[OperatorStatusTransition{op.Status, StatusSuspended}] {
        return nil, NewOperatorError("suspend",
            fmt.Sprintf("cannot transition from %s to SUSPENDED", op.Status))
    }
    
    op.Status = StatusSuspended
    if op.Metadata == nil {
        op.Metadata = make(map[string]string)
    }
    op.Metadata["suspension_reason"] = reason
    
    return r.UpdateOperator(op)
}

// ValidateOperator checks operator against capability registry.
func (r *InMemoryRegistry) ValidateOperator(op *Operator) *OperatorValidationResult {
    return r.validator.ValidateOperator(op)
}

// GetStats returns registry statistics.
func (r *InMemoryRegistry) GetStats() *OperatorRegistryStats {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    stats := &OperatorRegistryStats{
        ByStatus:     make(map[string]int),
        ByCapability: make(map[string]int),
    }
    
    totalCaps := 0
    for _, op := range r.operators {
        stats.TotalOperators++
        stats.ByStatus[string(op.Status)]++
        totalCaps += len(op.Capabilities)
        
        for _, cap := range op.Capabilities {
            stats.ByCapability[cap]++
        }
    }
    
    if stats.TotalOperators > 0 {
        stats.AverageCapabilities = float64(totalCaps) / float64(stats.TotalOperators)
    }
    
    return stats
}

// Helper
func timePtr(t time.Time) *time.Time {
    return &t
}
```

---

## Validator & Lifecycle Management

### Capability Validation & State Machine

```go
// internal/operator/validator.go

package operator

import (
    "fmt"
    "regexp"
    
    "internal/capability"
)

// OperatorValidator validates operators against capability registry.
type OperatorValidator struct {
    capRegistry capability.Registry
    hexRx       *regexp.Regexp
    alphanumRx  *regexp.Regexp
}

// NewOperatorValidator creates validator with capability registry.
func NewOperatorValidator(capRegistry capability.Registry) *OperatorValidator {
    return &OperatorValidator{
        capRegistry: capRegistry,
        hexRx:       regexp.MustCompile(`^[0-9a-fA-F]*$`),
        alphanumRx:  regexp.MustCompile(`^[a-zA-Z0-9_-]+$`),
    }
}

// ValidateOperator checks operator validity and capability requirements.
func (ov *OperatorValidator) ValidateOperator(op *Operator) *OperatorValidationResult {
    result := &OperatorValidationResult{IsValid: true}
    
    // Basic field validation
    if op.ID == "" {
        result.IsValid = false
        result.Errors = append(result.Errors, "ID required")
    } else if !ov.alphanumRx.MatchString(op.ID) {
        result.IsValid = false
        result.Errors = append(result.Errors, "ID must be alphanumeric")
    }
    
    if op.Address == "" {
        result.IsValid = false
        result.Errors = append(result.Errors, "Address required")
    } else if len(op.Address) != 40 {
        result.IsValid = false
        result.Errors = append(result.Errors, "Address must be 40 hex characters")
    } else if !ov.hexRx.MatchString(op.Address) {
        result.IsValid = false
        result.Errors = append(result.Errors, "Address must be valid hex")
    }
    
    if len(op.Capabilities) == 0 {
        result.IsValid = false
        result.Errors = append(result.Errors, "At least one capability required")
        return result
    }
    
    // Validate capabilities exist in registry
    for _, capID := range op.Capabilities {
        cap, err := ov.capRegistry.FindByID(capID)
        if err != nil {
            result.IsValid = false
            result.InvalidCapabilities = append(result.InvalidCapabilities, capID)
            result.Errors = append(result.Errors,
                fmt.Sprintf("Capability %s not found in registry", capID))
        } else if cap.Status != capability.StatusActive {
            result.IsValid = false
            result.Errors = append(result.Errors,
                fmt.Sprintf("Capability %s is not active", capID))
        }
    }
    
    return result
}

// OperatorError represents operator operation errors.
type OperatorError struct {
    Code    string
    Context string
    Message string
}

func (e OperatorError) Error() string {
    return fmt.Sprintf("[%s:%s] %s", e.Code, e.Context, e.Message)
}

func NewOperatorError(context string, msg string) OperatorError {
    return OperatorError{
        Code:    "OP_ERROR",
        Context: context,
        Message: msg,
    }
}

// Lifecycle validation

// CanTransition checks if transition is valid.
func (o *Operator) CanTransition(newStatus OperatorStatus) bool {
    return ValidTransitions[OperatorStatusTransition{o.Status, newStatus}]
}
```

---

## Testing Specification

```go
// internal/operator/registry_test.go

package operator

import (
    "testing"
    "time"
)

// TestRegisterOperator tests basic registration
func TestRegisterOperator(t *testing.T) {
    ledger := NewMockLedgerWriter()
    capReg := NewMockCapabilityRegistry()
    
    reg := NewInMemoryRegistry(ledger, capReg)
    
    op := &Operator{
        ID:           "validator-1",
        Name:         "Validator 1",
        Address:      "0000000000000000000000000000000000000001",
        Capabilities: []string{"validator"},
    }
    
    entry, err := reg.RegisterOperator(op)
    if err != nil {
        t.Fatalf("register failed: %v", err)
    }
    
    if entry == nil {
        t.Fatal("ledger entry not created")
    }
    
    // Verify can find
    found, _ := reg.FindByID(op.ID)
    if found.ID != op.ID {
        t.Fatal("operator not found")
    }
}

// TestDuplicateOperator tests duplicate prevention
func TestDuplicateOperator(t *testing.T) {
    ledger := NewMockLedgerWriter()
    capReg := NewMockCapabilityRegistry()
    reg := NewInMemoryRegistry(ledger, capReg)
    
    op := &Operator{
        ID:           "validator-1",
        Name:         "Validator 1",
        Address:      "0000000000000000000000000000000000000001",
        Capabilities: []string{"validator"},
    }
    
    reg.RegisterOperator(op)
    
    // Try to register same ID
    err := reg.RegisterOperator(op)
    if err == nil || err.Error() == "" {
        t.Fatal("expected duplicate error")
    }
}

// TestStateTransition tests lifecycle state machine
func TestStateTransition(t *testing.T) {
    ledger := NewMockLedgerWriter()
    capReg := NewMockCapabilityRegistry()
    reg := NewInMemoryRegistry(ledger, capReg)
    
    op := &Operator{
        ID:           "validator-1",
        Name:         "Validator 1",
        Address:      "0000000000000000000000000000000000000001",
        Capabilities: []string{"validator"},
    }
    
    reg.RegisterOperator(op)
    
    // Deactivate
    _, err := reg.DeactivateOperator("validator-1", "maintenance")
    if err != nil {
        t.Fatalf("deactivate failed: %v", err)
    }
    
    found, _ := reg.FindByID("validator-1")
    if found.Status != StatusInactive {
        t.Fatalf("expected INACTIVE, got %s", found.Status)
    }
    
    // Reactivate
    _, err = reg.ReactivateOperator("validator-1")
    if err != nil {
        t.Fatalf("reactivate failed: %v", err)
    }
    
    found, _ = reg.FindByID("validator-1")
    if found.Status != StatusActive {
        t.Fatalf("expected ACTIVE, got %s", found.Status)
    }
}

// TestCapabilityValidation tests capability requirement enforcement
func TestCapabilityValidation(t *testing.T) {
    ledger := NewMockLedgerWriter()
    capReg := NewMockCapabilityRegistry()
    
    // Setup: capability doesn't exist
    capReg.SetCapabilityStatus("nonexistent", false)
    
    reg := NewInMemoryRegistry(ledger, capReg)
    
    op := &Operator{
        ID:           "validator-1",
        Name:         "Validator 1",
        Address:      "0000000000000000000000000000000000000001",
        Capabilities: []string{"nonexistent"},
    }
    
    valResult := reg.ValidateOperator(op)
    if valResult.IsValid {
        t.Fatal("expected validation failure for missing capability")
    }
    
    if len(valResult.InvalidCapabilities) == 0 {
        t.Fatal("expected invalid capability list")
    }
}

// TestListByCapability tests capability-based discovery
func TestListByCapability(t *testing.T) {
    ledger := NewMockLedgerWriter()
    capReg := NewMockCapabilityRegistry()
    reg := NewInMemoryRegistry(ledger, capReg)
    
    // Register two operators with same capability
    for i := 1; i <= 2; i++ {
        op := &Operator{
            ID:           fmt.Sprintf("validator-%d", i),
            Name:         fmt.Sprintf("Validator %d", i),
            Address:      fmt.Sprintf("000000000000000000000000000000000000000%d", i),
            Capabilities: []string{"validator"},
        }
        reg.RegisterOperator(op)
    }
    
    ops, _ := reg.ListByCapability("validator")
    if len(ops) != 2 {
        t.Fatalf("expected 2 operators, got %d", len(ops))
    }
}

// Mock implementations for testing

func NewMockLedgerWriter() *MockLedgerWriter {
    return &MockLedgerWriter{entries: []*ledger.ChangeEntry{}}
}

type MockLedgerWriter struct {
    entries []*ledger.ChangeEntry
}

func (m *MockLedgerWriter) Append(entry *ledger.ChangeEntry) error {
    m.entries = append(m.entries, entry)
    entry.ID = uint64(len(m.entries))
    return nil
}

func (m *MockLedgerWriter) GetLastHash() (string, error) {
    if len(m.entries) == 0 {
        return "", nil
    }
    return m.entries[len(m.entries)-1].Hash, nil
}

func (m *MockLedgerWriter) GetLastID() (uint64, error) {
    return uint64(len(m.entries)), nil
}

func NewMockCapabilityRegistry() *MockCapabilityRegistry {
    return &MockCapabilityRegistry{
        capabilities: make(map[string]bool),
    }
}

type MockCapabilityRegistry struct {
    capabilities map[string]bool
}

func (m *MockCapabilityRegistry) SetCapabilityStatus(id string, exists bool) {
    m.capabilities[id] = exists
}

func (m *MockCapabilityRegistry) RegisterCapability(cap *capability.Capability) error {
    return nil
}

func (m *MockCapabilityRegistry) FindByID(id string) (*capability.Capability, error) {
    if !m.capabilities[id] {
        return nil, fmt.Errorf("not found")
    }
    return &capability.Capability{
        ID:     id,
        Status: capability.StatusActive,
    }, nil
}

// ... other mock methods
```

---

## Acceptance Criteria

- [ ] Operator type with ID, Address, Capabilities, Status, Metadata
- [ ] Registry with RegisterOperator, FindByID, FindByAddress, ListByCapability
- [ ] Lifecycle state machine (ACTIVE ↔ INACTIVE, SUSPENDED, REMOVED)
- [ ] Capability validation against L0-03 Capability Registry before registration
- [ ] Audit trail integration with L0-02 Change Ledger (all mutations recorded)
- [ ] 95%+ test coverage including state transitions and capability validation
- [ ] All tests passing with race detector
- [ ] Complete Go doc comments
- [ ] Integration test: Genesis → Ledger → Capabilities → Operators end-to-end

---

## End-to-End Integration Test Specification

This test validates the complete L0-01 through L1-01 flow:

```
1. Load Genesis (L0-01)
   ↓
2. Create Change Ledger (L0-02)
   ↓
3. Register Capabilities (L0-03)
   ↓
4. Register Operators with Capability Validation (L1-01)
   ↓
5. Query Audit Trail for all mutations
   ↓
6. Validate hash chains throughout
```

**Complete integration test location**: `tests/integration/foundation_test.go`

---

## Quality Gates Before L1-02

- [ ] All L0-01/02/03 tests passing
- [ ] L1-01 registration validated against actual Capability Registry
- [ ] Ledger entries created for all state mutations
- [ ] Operator address uniqueness enforced
- [ ] State transitions validated and restricted
- [ ] 95%+ test coverage across all four modules
- [ ] Race detector passes on all concurrent operations
- [ ] Manual end-to-end testing: genesis → ledger → capabilities → operators

---

## Next: L1-02 Bonding

Once L1-01 complete, L1-02 (Bonding) will:
- Allow operators to bond tokens to increase power
- Record bonding events in ledger
- Integrate with slashing engine

**Execute this. Complete L0-01 through L1-01 by EOD.** 🚀
