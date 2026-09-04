# L0-03: Capability Registry — Production Specification

**Task ID**: L0-03  
**Epic**: EPIC-1 (Core Protocol & Kernel)  
**Status**: Ready for Implementation  
**Estimated Duration**: 1-2 days  
**Prerequisites**: L0-01, L0-02  
**Blocking**: L1-01 (Operator Registry depends on capability validation)  
**Related Proposal**: X3-PROPOSAL-001

---

## Executive Summary

The Capability Registry is a type-safe, version-aware service for defining and discovering permissions in X3. It:
- Defines capabilities with semantic versioning and dependencies
- Validates capability constraints (circular dependencies, version ranges)
- Provides efficient capability lookup and discovery
- Resolves dependency graphs with topological sorting
- Integrates audit trail recording via L0-02 Change Ledger

**Design Principle**: Capabilities are immutable once registered. New versions are new registrations.

---

## Domain Types

### Capability Entity Model

```go
// internal/capability/types.go

package capability

import (
    "crypto/sha256"
    "encoding/hex"
    "encoding/json"
    "fmt"
    "time"
)

// Capability represents a permission or privilege in the system.
type Capability struct {
    // ID is unique capability identifier (e.g., "read-genesis", "slash-operator")
    ID string `json:"id" validate:"required,alphanum,min=3,max=64"`
    
    // Name is human-readable name
    Name string `json:"name" validate:"required,min=3,max=128"`
    
    // Version follows semantic versioning
    Version string `json:"version" validate:"required,semver"`
    
    // Description explains what capability grants
    Description string `json:"description" validate:"required,min=10"`
    
    // Permissions are low-level actions this capability grants
    Permissions []string `json:"permissions" validate:"required,min=1,dive"`
    
    // Dependencies are other capabilities required to use this one
    Dependencies []CapabilityRef `json:"dependencies,omitempty" validate:"dive"`
    
    // Constraints are optional restrictions on capability usage
    Constraints map[string]interface{} `json:"constraints,omitempty"`
    
    // Status indicates if capability is enabled or deprecated
    Status CapabilityStatus `json:"status" validate:"required"`
    
    // CreatedAt is creation timestamp
    CreatedAt time.Time `json:"createdAt" validate:"required"`
    
    // DeprecatedAt indicates when capability became deprecated (if relevant)
    DeprecatedAt *time.Time `json:"deprecatedAt,omitempty"`
    
    // Hash is SHA256 of normalized capability (computed)
    Hash string `json:"hash,omitempty"`
}

// CapabilityRef is reference to another capability with version constraints.
type CapabilityRef struct {
    // CapabilityID is the ID of the dependency
    CapabilityID string `json:"capabilityId" validate:"required,alphanum"`
    
    // MinVersion is minimum version (inclusive, semver)
    MinVersion string `json:"minVersion,omitempty" validate:"semver"`
    
    // MaxVersion is maximum version (inclusive, semver)
    MaxVersion string `json:"maxVersion,omitempty" validate:"semver"`
    
    // Required indicates if dependency is mandatory (vs optional)
    Required bool `json:"required"`
}

// CapabilityStatus indicates capability lifecycle.
type CapabilityStatus string

const (
    StatusActive      CapabilityStatus = "ACTIVE"
    StatusDeprecated  CapabilityStatus = "DEPRECATED"
    StatusRemoved     CapabilityStatus = "REMOVED"
)

// ComputeHash returns SHA256 of normalized capability.
func (c *Capability) ComputeHash() (string, error) {
    copy := *c
    copy.Hash = ""
    
    normalized, err := json.Marshal(copy)
    if err != nil {
        return "", fmt.Errorf("marshal: %w", err)
    }
    
    hash := sha256.Sum256(normalized)
    return hex.EncodeToString(hash[:]), nil
}

// DependencyGraph represents resolved capability dependencies.
type DependencyGraph struct {
    // RootID is the capability being resolved
    RootID string `json:"rootId"`
    
    // Nodes maps capability ID to capability
    Nodes map[string]*Capability `json:"nodes"`
    
    // Edges maps from capability ID to list of dependencies
    Edges map[string][]string `json:"edges"`
    
    // Order is topologically sorted capability IDs (safe to initialize in this order)
    Order []string `json:"order"`
    
    // IsValid indicates if graph has no cycles
    IsValid bool `json:"isValid"`
}

// RegistryStats contains capability registry statistics.
type RegistryStats struct {
    // TotalCapabilities is total count
    TotalCapabilities int `json:"totalCapabilities"`
    
    // ByStatus maps status to count
    ByStatus map[string]int `json:"byStatus"`
    
    // ByPermission maps permission to count of capabilities granting it
    ByPermission map[string]int `json:"byPermission"`
    
    // AverageDependencies is mean dependency count
    AverageDependencies float64 `json:"averageDependencies"`
}
```

---

## Registry Implementation

### Core Registry Operations

```go
// internal/capability/registry.go

package capability

import (
    "encoding/json"
    "fmt"
    "sync"
    "time"
)

// Registry manages capability definitions and discovery.
type Registry interface {
    // RegisterCapability adds new capability
    RegisterCapability(cap *Capability) error
    
    // FindByID returns capability by ID
    FindByID(id string) (*Capability, error)
    
    // FindByVersion returns specific version
    FindByVersion(id, version string) (*Capability, error)
    
    // FindByPermission returns all capabilities granting permission
    FindByPermission(permission string) ([]*Capability, error)
    
    // ListAll returns all active capabilities
    ListAll() ([]*Capability, error)
    
    // GetDependencyGraph returns resolved dependencies
    GetDependencyGraph(id string) (*DependencyGraph, error)
    
    // GetStats returns registry statistics
    GetStats() *RegistryStats
}

// InMemoryRegistry implements Registry with in-memory storage.
// For production, would integrate with L0-02 ledger.
type InMemoryRegistry struct {
    mu           sync.RWMutex
    capabilities map[string]map[string]*Capability // id -> version -> capability
    validator    *CapabilityValidator
    permIndex    map[string][]string // permission -> [ids]
}

// NewInMemoryRegistry creates new registry.
func NewInMemoryRegistry() *InMemoryRegistry {
    return &InMemoryRegistry{
        capabilities: make(map[string]map[string]*Capability),
        validator:    NewCapabilityValidator(),
        permIndex:    make(map[string][]string),
    }
}

// RegisterCapability validates and registers new capability.
func (r *InMemoryRegistry) RegisterCapability(cap *Capability) error {
    if cap == nil {
        return NewCapabilityError("register", "capability is nil")
    }
    
    // Validate capability
    errs := r.validator.ValidateCapability(cap)
    if len(errs) > 0 {
        return NewCapabilityError("validate", fmt.Sprintf("%v", errs))
    }
    
    r.mu.Lock()
    defer r.mu.Unlock()
    
    // Check for duplicate
    if versions, ok := r.capabilities[cap.ID]; ok {
        if _, exists := versions[cap.Version]; exists {
            return NewCapabilityError("register", 
                fmt.Sprintf("capability %s v%s already exists", cap.ID, cap.Version))
        }
    }
    
    // Compute hash
    hash, err := cap.ComputeHash()
    if err != nil {
        return err
    }
    cap.Hash = hash
    cap.CreatedAt = time.Now().UTC()
    
    // Store
    if r.capabilities[cap.ID] == nil {
        r.capabilities[cap.ID] = make(map[string]*Capability)
    }
    r.capabilities[cap.ID][cap.Version] = cap
    
    // Update permission index
    for _, perm := range cap.Permissions {
        found := false
        for _, id := range r.permIndex[perm] {
            if id == cap.ID {
                found = true
                break
            }
        }
        if !found {
            r.permIndex[perm] = append(r.permIndex[perm], cap.ID)
        }
    }
    
    return nil
}

// FindByID returns latest version of capability.
func (r *InMemoryRegistry) FindByID(id string) (*Capability, error) {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    versions, ok := r.capabilities[id]
    if !ok || len(versions) == 0 {
        return nil, NewCapabilityError("find", 
            fmt.Sprintf("capability %s not found", id))
    }
    
    // Return latest version (highest semver)
    var latest *Capability
    for _, cap := range versions {
        if latest == nil || isNewerVersion(cap.Version, latest.Version) {
            latest = cap
        }
    }
    
    return latest, nil
}

// FindByVersion returns specific version.
func (r *InMemoryRegistry) FindByVersion(id, version string) (*Capability, error) {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    versions, ok := r.capabilities[id]
    if !ok {
        return nil, NewCapabilityError("find",
            fmt.Sprintf("capability %s not found", id))
    }
    
    cap, ok := versions[version]
    if !ok {
        return nil, NewCapabilityError("find",
            fmt.Sprintf("capability %s v%s not found", id, version))
    }
    
    return cap, nil
}

// FindByPermission returns all capabilities granting permission.
func (r *InMemoryRegistry) FindByPermission(permission string) ([]*Capability, error) {
    r.mu.RLock()
    ids := r.permIndex[permission]
    r.mu.RUnlock()
    
    if len(ids) == 0 {
        return nil, NewCapabilityError("find",
            fmt.Sprintf("no capabilities grant permission %q", permission))
    }
    
    var results []*Capability
    for _, id := range ids {
        cap, _ := r.FindByID(id)
        if cap != nil {
            results = append(results, cap)
        }
    }
    
    return results, nil
}

// ListAll returns all active capabilities.
func (r *InMemoryRegistry) ListAll() ([]*Capability, error) {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    var capabilities []*Capability
    seen := make(map[string]bool)
    
    for id, versions := range r.capabilities {
        if seen[id] {
            continue
        }
        seen[id] = true
        
        // Get latest version
        var latest *Capability
        for _, cap := range versions {
            if latest == nil || isNewerVersion(cap.Version, latest.Version) {
                latest = cap
            }
        }
        
        if latest != nil && latest.Status == StatusActive {
            capabilities = append(capabilities, latest)
        }
    }
    
    return capabilities, nil
}

// GetDependencyGraph returns resolved dependencies.
func (r *InMemoryRegistry) GetDependencyGraph(id string) (*DependencyGraph, error) {
    cap, err := r.FindByID(id)
    if err != nil {
        return nil, err
    }
    
    graph := &DependencyGraph{
        RootID: id,
        Nodes:  make(map[string]*Capability),
        Edges:  make(map[string][]string),
        IsValid: true,
    }
    
    // Resolve all dependencies recursively
    visited := make(map[string]bool)
    if err := r.resolveDeps(cap, graph, visited); err != nil {
        graph.IsValid = false
        return graph, err
    }
    
    // Topological sort
    graph.Order = topologicalSort(graph.Edges)
    
    return graph, nil
}

// GetStats returns registry statistics.
func (r *InMemoryRegistry) GetStats() *RegistryStats {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    stats := &RegistryStats{
        ByStatus:    make(map[string]int),
        ByPermission: make(map[string]int),
    }
    
    totalDeps := 0
    for id, versions := range r.capabilities {
        for _, cap := range versions {
            stats.TotalCapabilities++
            stats.ByStatus[string(cap.Status)]++
            totalDeps += len(cap.Dependencies)
            
            for _, perm := range cap.Permissions {
                stats.ByPermission[perm]++
            }
        }
    }
    
    if stats.TotalCapabilities > 0 {
        stats.AverageDependencies = float64(totalDeps) / float64(stats.TotalCapabilities)
    }
    
    return stats
}

// resolveDeps recursively resolves dependencies
func (r *InMemoryRegistry) resolveDeps(cap *Capability, graph *DependencyGraph, 
    visited map[string]bool) error {
    
    if visited[cap.ID] {
        return NewCapabilityError("dependency", 
            fmt.Sprintf("circular dependency detected at %s", cap.ID))
    }
    
    visited[cap.ID] = true
    graph.Nodes[cap.ID] = cap
    
    for _, dep := range cap.Dependencies {
        depCap, err := r.FindByID(dep.CapabilityID)
        if err != nil {
            return err
        }
        
        graph.Edges[cap.ID] = append(graph.Edges[cap.ID], dep.CapabilityID)
        
        // Recurse
        if err := r.resolveDeps(depCap, graph, visited); err != nil {
            return err
        }
    }
    
    visited[cap.ID] = false
    return nil
}

// Helper functions

func isNewerVersion(v1, v2 string) bool {
    // Simplified: compare as strings
    // Production would use proper semver comparison
    return v1 > v2
}

func topologicalSort(edges map[string][]string) []string {
    // Kahn's algorithm
    inDegree := make(map[string]int)
    for node := range edges {
        if _, ok := inDegree[node]; !ok {
            inDegree[node] = 0
        }
    }
    
    for _, deps := range edges {
        for _, dep := range deps {
            inDegree[dep]++
        }
    }
    
    queue := []string{}
    for node, deg := range inDegree {
        if deg == 0 {
            queue = append(queue, node)
        }
    }
    
    var sorted []string
    for len(queue) > 0 {
        node := queue[0]
        queue = queue[1:]
        sorted = append(sorted, node)
        
        for _, dep := range edges[node] {
            inDegree[dep]--
            if inDegree[dep] == 0 {
                queue = append(queue, dep)
            }
        }
    }
    
    return sorted
}
```

---

## Validator & Error Handling

### Capability Validation

```go
// internal/capability/validator.go

package capability

import (
    "fmt"
    "regexp"
)

// CapabilityValidator validates capability definitions.
type CapabilityValidator struct {
    semverRx   *regexp.Regexp
    alphanumRx *regexp.Regexp
}

// NewCapabilityValidator creates new validator.
func NewCapabilityValidator() *CapabilityValidator {
    return &CapabilityValidator{
        semverRx:   regexp.MustCompile(`^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$`),
        alphanumRx: regexp.MustCompile(`^[a-zA-Z0-9_-]+$`),
    }
}

// ValidateCapability validates complete capability.
func (cv *CapabilityValidator) ValidateCapability(cap *Capability) []ValidationError {
    var errs []ValidationError
    
    // ID validation
    if cap.ID == "" {
        errs = append(errs, NewValidationError("ID", "", "required", "ID required"))
    } else if !cv.alphanumRx.MatchString(cap.ID) {
        errs = append(errs, NewValidationError("ID", cap.ID, "format", "ID must be alphanumeric"))
    } else if len(cap.ID) < 3 || len(cap.ID) > 64 {
        errs = append(errs, NewValidationError("ID", cap.ID, "length", "ID must be 3-64 characters"))
    }
    
    // Version validation
    if cap.Version == "" {
        errs = append(errs, NewValidationError("Version", "", "required", "Version required"))
    } else if !cv.semverRx.MatchString(cap.Version) {
        errs = append(errs, NewValidationError("Version", cap.Version, "semver", "Version must be semver"))
    }
    
    // Permissions validation
    if len(cap.Permissions) == 0 {
        errs = append(errs, NewValidationError("Permissions", nil, "min=1", "At least one permission required"))
    }
    
    // Dependencies validation
    for i, dep := range cap.Dependencies {
        if dep.CapabilityID == "" {
            errs = append(errs, NewValidationError(
                fmt.Sprintf("Dependencies[%d].CapabilityID", i), "", "required", "Dependency ID required"))
        }
    }
    
    // Status validation
    if cap.Status != StatusActive && cap.Status != StatusDeprecated && cap.Status != StatusRemoved {
        errs = append(errs, NewValidationError("Status", cap.Status, "enum", "Invalid status"))
    }
    
    return errs
}

// ValidationError represents validation failure.
type ValidationError struct {
    Field      string
    Value      interface{}
    Constraint string
    Message    string
}

func NewValidationError(field string, value interface{}, constraint string, msg string) ValidationError {
    return ValidationError{
        Field:      field,
        Value:      value,
        Constraint: constraint,
        Message:    msg,
    }
}

// CapabilityError represents capability operation errors.
type CapabilityError struct {
    Code    string
    Context string
    Message string
}

func (e CapabilityError) Error() string {
    return fmt.Sprintf("[%s:%s] %s", e.Code, e.Context, e.Message)
}

func NewCapabilityError(context string, msg string) CapabilityError {
    return CapabilityError{
        Code:    "CAP_ERROR",
        Context: context,
        Message: msg,
    }
}
```

---

## Testing Specification

```go
// internal/capability/registry_test.go

package capability

import (
    "testing"
    "time"
)

// TestRegisterCapability tests basic registration
func TestRegisterCapability(t *testing.T) {
    reg := NewInMemoryRegistry()
    
    cap := &Capability{
        ID:          "read-genesis",
        Name:        "Read Genesis",
        Version:     "1.0.0",
        Description: "Capability to read genesis state",
        Permissions: []string{"read:genesis"},
        Status:      StatusActive,
    }
    
    err := reg.RegisterCapability(cap)
    if err != nil {
        t.Fatalf("register failed: %v", err)
    }
    
    // Verify can find it
    found, _ := reg.FindByID(cap.ID)
    if found.ID != cap.ID {
        t.Fatalf("capability not found")
    }
}

// TestDuplicateRegistration tests duplicate prevention
func TestDuplicateRegistration(t *testing.T) {
    reg := NewInMemoryRegistry()
    
    cap := &Capability{
        ID:          "test",
        Name:        "Test",
        Version:     "1.0.0",
        Description: "Test capability",
        Permissions: []string{"test"},
        Status:      StatusActive,
    }
    
    _ = reg.RegisterCapability(cap)
    
    // Try to register same version again
    err := reg.RegisterCapability(cap)
    if err == nil {
        t.Fatal("expected duplicate error")
    }
}

// TestCircularDependency tests circular dependency detection
func TestCircularDependency(t *testing.T) {
    reg := NewInMemoryRegistry()
    
    // Create A -> B -> A cycle
    capA := &Capability{
        ID:          "cap-a",
        Name:        "Cap A",
        Version:     "1.0.0",
        Description: "Cap A",
        Permissions: []string{"a"},
        Dependencies: []CapabilityRef{
            {CapabilityID: "cap-b", Required: true},
        },
        Status: StatusActive,
    }
    
    capB := &Capability{
        ID:          "cap-b",
        Name:        "Cap B",
        Version:     "1.0.0",
        Description: "Cap B",
        Permissions: []string{"b"},
        Dependencies: []CapabilityRef{
            {CapabilityID: "cap-a", Required: true},
        },
        Status: StatusActive,
    }
    
    reg.RegisterCapability(capA)
    reg.RegisterCapability(capB)
    
    // Try to get dependency graph - should detect cycle
    graph, err := reg.GetDependencyGraph("cap-a")
    if err == nil && graph.IsValid {
        t.Fatal("expected circular dependency detection")
    }
}

// TestDependencyResolution tests dependency graph building
func TestDependencyResolution(t *testing.T) {
    reg := NewInMemoryRegistry()
    
    // Create A -> B, C -> D
    capA := &Capability{
        ID:          "cap-a",
        Name:        "Cap A",
        Version:     "1.0.0",
        Description: "Cap A",
        Permissions: []string{"a"},
        Dependencies: []CapabilityRef{
            {CapabilityID: "cap-b", Required: true},
        },
        Status: StatusActive,
    }
    
    capB := &Capability{
        ID:          "cap-b",
        Name:        "Cap B",
        Version:     "1.0.0",
        Description: "Cap B",
        Permissions: []string{"b"},
        Dependencies: []CapabilityRef{
            {CapabilityID: "cap-d", Required: true},
        },
        Status: StatusActive,
    }
    
    capD := &Capability{
        ID:          "cap-d",
        Name:        "Cap D",
        Version:     "1.0.0",
        Description: "Cap D",
        Permissions: []string{"d"},
        Status:      StatusActive,
    }
    
    reg.RegisterCapability(capA)
    reg.RegisterCapability(capB)
    reg.RegisterCapability(capD)
    
    graph, _ := reg.GetDependencyGraph("cap-a")
    if !graph.IsValid {
        t.Fatal("graph should be valid")
    }
    
    if len(graph.Nodes) != 3 {
        t.Fatalf("expected 3 nodes, got %d", len(graph.Nodes))
    }
}

// TestFindByPermission tests permission lookup
func TestFindByPermission(t *testing.T) {
    reg := NewInMemoryRegistry()
    
    cap := &Capability{
        ID:          "read-caps",
        Name:        "Read Capabilities",
        Version:     "1.0.0",
        Description: "Read capabilities",
        Permissions: []string{"read:capability", "list:capability"},
        Status:      StatusActive,
    }
    
    reg.RegisterCapability(cap)
    
    cps, _ := reg.FindByPermission("read:capability")
    if len(cps) != 1 || cps[0].ID != cap.ID {
        t.Fatal("permission lookup failed")
    }
}
```

---

## Acceptance Criteria

- [ ] Capability type with ID, Version, Permissions, Dependencies, Status
- [ ] Registry with RegisterCapability, FindByID, FindByVersion, FindByPermission
- [ ] Circular dependency detection with clear error messages
- [ ] Dependency graph resolution with topological sort
- [ ] Version constraint validation
- [ ] 95%+ test coverage including dependency edge cases
- [ ] All tests passing with race detector
- [ ] Complete Go doc comments
- [ ] Integration with L0-02 ledger for audit trail

**Execute.** 🚀
