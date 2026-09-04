# L0-01: Genesis Configuration Engine — Production Specification

**Task ID**: L0-01  
**Epic**: EPIC-1 (Core Protocol & Kernel)  
**Status**: Ready for Implementation  
**Estimated Duration**: 1-2 days  
**Blocking**: All subsequent L0/L1 tasks  
**Related Proposal**: X3-PROPOSAL-001

---

## Executive Summary

The Genesis Configuration Engine is the immutable, versioned configuration source for the X3 blockchain. It:
- Defines initial chain state (validators, accounts, consensus parameters)
- Enforces schema validation with deterministic version management
- Provides migration pathways between schema versions
- Creates audit-trail compatibility with L0-02 Change Ledger
- Serves as foundation for L0-05 Local Chain Simulator

**Success Criterion**: Genesis configuration is reproducible, immutable, and deterministically validatable across all runtimes.

---

## Architecture Overview

### Clean Architecture Layers

```
┌─────────────────────────────────────────┐
│  Presentation / API Layer (cmd/x3-genesis) │
│  - CLI: load, validate, migrate genesis  │
│  - JSON/YAML endpoints                  │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│  Application Layer (use cases)           │
│  - ValidateGenesisUseCase               │
│  - MigrateGenesisUseCase                │
│  - LoadGenesisUseCase                   │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│  Domain Layer (entities, no deps)        │
│  - Genesis (entity)                     │
│  - Validator (entity)                   │
│  - Account (entity)                     │
│  - ConsensusParams (entity)             │
│  - ValidationError (domain exception)   │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│  Infrastructure Layer                    │
│  - genesis/schema.go (schema defs)      │
│  - genesis/validator.go (validation)    │
│  - genesis/migration.go (migrations)    │
│  - genesis/storage.go (file I/O)        │
└─────────────────────────────────────────┘
```

---

## Domain Types (Zero External Dependencies)

### Core Genesis Entity

```go
// internal/genesis/types.go

package genesis

import (
    "crypto/sha256"
    "encoding/hex"
    "fmt"
    "time"
)

// Genesis represents the immutable initial blockchain state.
// All fields are required and immutable after creation.
type Genesis struct {
    // Version follows semantic versioning (e.g., "1.0.0", "1.1.0")
    Version string `json:"version" validate:"required,semver"`
    
    // ChainID uniquely identifies this blockchain (e.g., "x3-mainnet-1")
    ChainID string `json:"chainId" validate:"required,min=3,max=32,alphanum"`
    
    // Timestamp is creation time in ISO 8601 UTC
    Timestamp time.Time `json:"timestamp" validate:"required"`
    
    // ConsensusParams defines consensus algorithm configuration
    ConsensusParams *ConsensusParams `json:"consensusParams" validate:"required"`
    
    // Validators are the initial validator set
    Validators []Validator `json:"validators" validate:"required,min=1,dive"`
    
    // Accounts are initial account balances
    Accounts []Account `json:"accounts" validate:"dive"`
    
    // AppState contains module-specific initialization data
    AppState map[string]interface{} `json:"appState" validate:"required"`
    
    // Hash is SHA256 of normalized JSON (computed, not in input)
    Hash string `json:"hash,omitempty"`
}

// ConsensusParams defines BFT consensus parameters.
type ConsensusParams struct {
    // BlockSize in bytes
    BlockSize *BlockSizeParams `json:"blockSize" validate:"required"`
    
    // Evidence validity period
    EvidenceParams *EvidenceParams `json:"evidenceParams" validate:"required"`
    
    // Validator update parameters
    ValidatorParams *ValidatorParams `json:"validatorParams" validate:"required"`
}

// BlockSizeParams defines block constraints.
type BlockSizeParams struct {
    MaxBytes int64 `json:"maxBytes" validate:"required,gt=0,lt=100000000"`
    MaxGas   int64 `json:"maxGas" validate:"required,gt=0"`
}

// EvidenceParams defines equivocation handling.
type EvidenceParams struct {
    MaxAgeNumBlocks int64         `json:"maxAgeNumBlocks" validate:"required,gt=0"`
    MaxAgeDuration  time.Duration `json:"maxAgeDuration" validate:"required"`
}

// ValidatorParams defines validator set rules.
type ValidatorParams struct {
    PubKeyTypes []string `json:"pubKeyTypes" validate:"required,min=1,dive"`
}

// Validator represents an initial validator in the genesis set.
type Validator struct {
    // Address is hex-encoded validator address
    Address string `json:"address" validate:"required,hexadecimal,len=40"`
    
    // PublicKey is hex-encoded Ed25519 or Secp256k1 public key
    PublicKey string `json:"publicKey" validate:"required,hexadecimal,min=64,max=132"`
    
    // Power is voting power (must be > 0)
    Power int64 `json:"power" validate:"required,gt=0"`
    
    // Name is optional validator moniker
    Name string `json:"name,omitempty" validate:"max=64"`
}

// Account represents an initial account with balances.
type Account struct {
    // Address is account identifier (hex-encoded)
    Address string `json:"address" validate:"required,hexadecimal,len=40"`
    
    // Balances maps token denomination to amount
    Balances map[string]uint64 `json:"balances" validate:"required,min=1,dive"`
    
    // Nonce prevents replay attacks
    Nonce uint64 `json:"nonce,omitempty" validate:"gte=0"`
    
    // Name is optional account moniker
    Name string `json:"name,omitempty" validate:"max=64"`
}

// ComputeHash returns SHA256 hash of normalized genesis JSON.
// Normalization: sorted keys, no whitespace, UTC timestamps.
func (g *Genesis) ComputeHash() (string, error) {
    normalized, err := normalizeGenesisJSON(g)
    if err != nil {
        return "", fmt.Errorf("normalize genesis: %w", err)
    }
    
    hash := sha256.Sum256(normalized)
    return hex.EncodeToString(hash[:]), nil
}

// IsValid checks structural validity (consensus params, validator set, etc.)
func (g *Genesis) IsValid() error {
    if g.Version == "" {
        return NewValidationError("Version", "", "required", "Genesis version must be specified")
    }
    
    if len(g.Validators) == 0 {
        return NewValidationError("Validators", nil, "min=1", "At least one validator required")
    }
    
    totalPower := int64(0)
    for i, v := range g.Validators {
        if v.Power <= 0 {
            return NewValidationError(
                fmt.Sprintf("Validators[%d].Power", i),
                v.Power,
                "gt=0",
                fmt.Sprintf("Validator power must be positive, got %d", v.Power),
            )
        }
        totalPower += v.Power
    }
    
    if totalPower == 0 {
        return NewValidationError("Validators", nil, "power_sum", "Total validator power must be > 0")
    }
    
    return nil
}

// ValidationError represents a schema validation failure.
type ValidationError struct {
    Field       string      // e.g., "Validators[0].Address"
    Value       interface{} // actual value that failed
    Constraint  string      // e.g., "required", "hexadecimal"
    Message     string      // human-friendly error
}

func (e ValidationError) Error() string {
    return fmt.Sprintf("validation error on field '%s': %s", e.Field, e.Message)
}

// NewValidationError creates a validation error with remediation hint.
func NewValidationError(field string, value interface{}, constraint string, msg string) ValidationError {
    return ValidationError{
        Field:      field,
        Value:      value,
        Constraint: constraint,
        Message:    msg,
    }
}
```

---

## Schema Validation

### Validator Implementation

```go
// internal/genesis/validator.go

package genesis

import (
    "fmt"
    "regexp"
    "strings"
    "time"
)

// SchemaValidator validates Genesis against constraints.
type SchemaValidator struct {
    // Custom validators for special cases
    semverRx   *regexp.Regexp
    hexRx      *regexp.Regexp
    alphanumRx *regexp.Regexp
}

// NewSchemaValidator creates validator with compiled regexes.
func NewSchemaValidator() *SchemaValidator {
    return &SchemaValidator{
        semverRx:   regexp.MustCompile(`^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$`),
        hexRx:      regexp.MustCompile(`^[0-9a-fA-F]*$`),
        alphanumRx: regexp.MustCompile(`^[a-zA-Z0-9_-]+$`),
    }
}

// ValidateGenesisSchema validates complete Genesis structure.
func (v *SchemaValidator) ValidateGenesisSchema(g *Genesis) []ValidationError {
    var errs []ValidationError
    
    // Version validation (semver)
    if g.Version == "" {
        errs = append(errs, NewValidationError(
            "Version", "", "required",
            "Genesis version is required and must follow semantic versioning (e.g., 1.0.0)",
        ))
    } else if !v.semverRx.MatchString(g.Version) {
        errs = append(errs, NewValidationError(
            "Version", g.Version, "semver",
            fmt.Sprintf("Version must be semver (X.Y.Z), got %q", g.Version),
        ))
    }
    
    // ChainID validation
    if g.ChainID == "" {
        errs = append(errs, NewValidationError(
            "ChainID", "", "required",
            "ChainID is required",
        ))
    } else if len(g.ChainID) < 3 || len(g.ChainID) > 32 {
        errs = append(errs, NewValidationError(
            "ChainID", g.ChainID, "length",
            fmt.Sprintf("ChainID must be 3-32 characters, got %d", len(g.ChainID)),
        ))
    }
    
    // Timestamp validation
    if g.Timestamp.IsZero() {
        errs = append(errs, NewValidationError(
            "Timestamp", g.Timestamp, "required",
            "Timestamp is required",
        ))
    }
    
    // ConsensusParams validation
    if g.ConsensusParams == nil {
        errs = append(errs, NewValidationError(
            "ConsensusParams", nil, "required",
            "ConsensusParams is required",
        ))
    } else {
        errs = append(errs, v.validateConsensusParams(g.ConsensusParams)...)
    }
    
    // Validators validation
    if len(g.Validators) == 0 {
        errs = append(errs, NewValidationError(
            "Validators", nil, "min=1",
            "At least one validator is required",
        ))
    } else {
        errs = append(errs, v.validateValidators(g.Validators)...)
    }
    
    // Accounts validation (optional)
    if len(g.Accounts) > 0 {
        errs = append(errs, v.validateAccounts(g.Accounts)...)
    }
    
    // AppState validation (must be non-nil)
    if g.AppState == nil {
        errs = append(errs, NewValidationError(
            "AppState", nil, "required",
            "AppState is required (can be empty map)",
        ))
    }
    
    return errs
}

// validateConsensusParams validates consensus parameters.
func (v *SchemaValidator) validateConsensusParams(cp *ConsensusParams) []ValidationError {
    var errs []ValidationError
    
    if cp.BlockSize == nil {
        errs = append(errs, NewValidationError(
            "ConsensusParams.BlockSize", nil, "required",
            "BlockSize is required",
        ))
    } else {
        if cp.BlockSize.MaxBytes <= 0 || cp.BlockSize.MaxBytes > 100_000_000 {
            errs = append(errs, NewValidationError(
                "ConsensusParams.BlockSize.MaxBytes",
                cp.BlockSize.MaxBytes,
                "range",
                fmt.Sprintf("MaxBytes must be 1-100,000,000, got %d", cp.BlockSize.MaxBytes),
            ))
        }
        if cp.BlockSize.MaxGas <= 0 {
            errs = append(errs, NewValidationError(
                "ConsensusParams.BlockSize.MaxGas",
                cp.BlockSize.MaxGas,
                "gt=0",
                fmt.Sprintf("MaxGas must be > 0, got %d", cp.BlockSize.MaxGas),
            ))
        }
    }
    
    if cp.EvidenceParams == nil {
        errs = append(errs, NewValidationError(
            "ConsensusParams.EvidenceParams", nil, "required",
            "EvidenceParams is required",
        ))
    } else {
        if cp.EvidenceParams.MaxAgeNumBlocks <= 0 {
            errs = append(errs, NewValidationError(
                "ConsensusParams.EvidenceParams.MaxAgeNumBlocks",
                cp.EvidenceParams.MaxAgeNumBlocks,
                "gt=0",
                "MaxAgeNumBlocks must be > 0",
            ))
        }
    }
    
    return errs
}

// validateValidators validates validator set.
func (v *SchemaValidator) validateValidators(validators []Validator) []ValidationError {
    var errs []ValidationError
    seenAddresses := make(map[string]bool)
    totalPower := int64(0)
    
    for i, val := range validators {
        prefix := fmt.Sprintf("Validators[%d]", i)
        
        // Address validation (40-char hex)
        if val.Address == "" {
            errs = append(errs, NewValidationError(
                prefix+".Address", "", "required",
                "Validator address is required",
            ))
        } else {
            if len(val.Address) != 40 {
                errs = append(errs, NewValidationError(
                    prefix+".Address", val.Address, "len=40",
                    fmt.Sprintf("Address must be 40 hex characters, got %d", len(val.Address)),
                ))
            }
            if !v.hexRx.MatchString(val.Address) {
                errs = append(errs, NewValidationError(
                    prefix+".Address", val.Address, "hexadecimal",
                    "Address must be valid hexadecimal",
                ))
            }
            if seenAddresses[val.Address] {
                errs = append(errs, NewValidationError(
                    prefix+".Address", val.Address, "unique",
                    "Duplicate validator address",
                ))
            }
            seenAddresses[val.Address] = true
        }
        
        // PublicKey validation
        if val.PublicKey == "" {
            errs = append(errs, NewValidationError(
                prefix+".PublicKey", "", "required",
                "Validator public key is required",
            ))
        } else {
            if !v.hexRx.MatchString(val.PublicKey) {
                errs = append(errs, NewValidationError(
                    prefix+".PublicKey", val.PublicKey, "hexadecimal",
                    "Public key must be valid hexadecimal",
                ))
            }
        }
        
        // Power validation
        if val.Power <= 0 {
            errs = append(errs, NewValidationError(
                prefix+".Power", val.Power, "gt=0",
                fmt.Sprintf("Validator power must be > 0, got %d", val.Power),
            ))
        }
        totalPower += val.Power
    }
    
    if totalPower == 0 {
        errs = append(errs, NewValidationError(
            "Validators", nil, "power_sum",
            "Total validator power must be > 0",
        ))
    }
    
    return errs
}

// validateAccounts validates account set.
func (v *SchemaValidator) validateAccounts(accounts []Account) []ValidationError {
    var errs []ValidationError
    seenAddresses := make(map[string]bool)
    
    for i, acc := range accounts {
        prefix := fmt.Sprintf("Accounts[%d]", i)
        
        // Address validation
        if acc.Address == "" {
            errs = append(errs, NewValidationError(
                prefix+".Address", "", "required",
                "Account address is required",
            ))
        } else {
            if len(acc.Address) != 40 {
                errs = append(errs, NewValidationError(
                    prefix+".Address", acc.Address, "len=40",
                    fmt.Sprintf("Address must be 40 hex characters, got %d", len(acc.Address)),
                ))
            }
            if seenAddresses[acc.Address] {
                errs = append(errs, NewValidationError(
                    prefix+".Address", acc.Address, "unique",
                    "Duplicate account address",
                ))
            }
            seenAddresses[acc.Address] = true
        }
        
        // Balances validation
        if len(acc.Balances) == 0 {
            errs = append(errs, NewValidationError(
                prefix+".Balances", nil, "min=1",
                "Account must have at least one balance entry",
            ))
        }
    }
    
    return errs
}
```

---

## Schema Versioning & Migrations

### Migration Framework

```go
// internal/genesis/migration.go

package genesis

import (
    "fmt"
)

// MigrationFunc is a function that migrates genesis from one version to another.
type MigrationFunc func(*Genesis) (*Genesis, error)

// MigrationRegistry manages version transitions.
type MigrationRegistry struct {
    migrations map[string]MigrationFunc // key: "v1.0.0->v1.1.0"
}

// NewMigrationRegistry creates empty migration registry.
func NewMigrationRegistry() *MigrationRegistry {
    return &MigrationRegistry{
        migrations: make(map[string]MigrationFunc),
    }
}

// RegisterMigration registers a migration from one version to next.
func (mr *MigrationRegistry) RegisterMigration(from, to string, fn MigrationFunc) error {
    if from == to {
        return fmt.Errorf("cannot migrate to same version: %s", from)
    }
    key := fmt.Sprintf("%s->%s", from, to)
    mr.migrations[key] = fn
    return nil
}

// Migrate transitions genesis from source version to target version.
// Returns error if no migration path exists.
func (mr *MigrationRegistry) Migrate(g *Genesis, targetVersion string) (*Genesis, error) {
    if g.Version == targetVersion {
        return g, nil // No migration needed
    }
    
    current := g
    path := findMigrationPath(g.Version, targetVersion, mr.migrations)
    
    if path == nil {
        return nil, fmt.Errorf("no migration path from %s to %s", g.Version, targetVersion)
    }
    
    for _, step := range path {
        key := fmt.Sprintf("%s->%s", current.Version, step)
        fn, ok := mr.migrations[key]
        if !ok {
            return nil, fmt.Errorf("migration not registered: %s", key)
        }
        
        migrated, err := fn(current)
        if err != nil {
            return nil, fmt.Errorf("migration %s failed: %w", key, err)
        }
        
        migrated.Version = step
        current = migrated
    }
    
    return current, nil
}

// findMigrationPath uses BFS to find shortest path between versions.
// For now, assumes linear path (v1.0.0 -> v1.1.0 -> v2.0.0, etc.)
// In production, would use graph algorithms for complex migration networks.
func findMigrationPath(from, to string, migrations map[string]MigrationFunc) []string {
    // Simplified: assume sequential migration only
    // Real implementation would build migration graph and use Dijkstra/BFS
    
    // This is a placeholder for demonstration
    // Production code would implement proper graph traversal
    return nil
}

// Built-in migrations

// MigrateV100ToV110 migrates from v1.0.0 to v1.1.0
// Example: Add new field with default value
func MigrateV100ToV110(g *Genesis) (*Genesis, error) {
    // Deep copy
    result := *g
    
    // Example migration: ensure AppState has required keys
    if result.AppState == nil {
        result.AppState = make(map[string]interface{})
    }
    
    // Ensure banking module state exists
    if _, ok := result.AppState["bank"]; !ok {
        result.AppState["bank"] = map[string]interface{}{
            "genesisState": map[string]interface{}{},
        }
    }
    
    return &result, nil
}
```

---

## Storage & I/O

### File Storage Implementation

```go
// internal/genesis/storage.go

package genesis

import (
    "encoding/json"
    "fmt"
    "io/ioutil"
    "os"
    "path/filepath"
)

// Storage interface abstracts genesis persistence.
type Storage interface {
    // Save persists genesis to storage
    Save(g *Genesis) error
    
    // Load retrieves genesis from storage
    Load() (*Genesis, error)
    
    // Version returns current stored genesis version
    Version() (string, error)
}

// FileStorage persists genesis to JSON file.
type FileStorage struct {
    path string // .../genesis.json
}

// NewFileStorage creates file-based storage.
func NewFileStorage(path string) (*FileStorage, error) {
    dir := filepath.Dir(path)
    if err := os.MkdirAll(dir, 0755); err != nil {
        return nil, fmt.Errorf("create directory: %w", err)
    }
    return &FileStorage{path: path}, nil
}

// Save writes genesis to file with atomic write (write-to-temp, then rename).
func (fs *FileStorage) Save(g *Genesis) error {
    if g == nil {
        return fmt.Errorf("genesis is nil")
    }
    
    // Compute hash before saving
    hash, err := g.ComputeHash()
    if err != nil {
        return fmt.Errorf("compute hash: %w", err)
    }
    g.Hash = hash
    
    // Marshal to JSON
    data, err := json.MarshalIndent(g, "", "  ")
    if err != nil {
        return fmt.Errorf("marshal JSON: %w", err)
    }
    
    // Write to temp file
    tmpFile := fs.path + ".tmp"
    if err := ioutil.WriteFile(tmpFile, data, 0644); err != nil {
        return fmt.Errorf("write temp file: %w", err)
    }
    
    // Atomic rename
    if err := os.Rename(tmpFile, fs.path); err != nil {
        os.Remove(tmpFile) // cleanup
        return fmt.Errorf("atomic rename: %w", err)
    }
    
    return nil
}

// Load reads genesis from file.
func (fs *FileStorage) Load() (*Genesis, error) {
    data, err := ioutil.ReadFile(fs.path)
    if err != nil {
        return nil, fmt.Errorf("read file: %w", err)
    }
    
    var g Genesis
    if err := json.Unmarshal(data, &g); err != nil {
        return nil, fmt.Errorf("unmarshal JSON: %w", err)
    }
    
    return &g, nil
}

// Version returns version of stored genesis.
func (fs *FileStorage) Version() (string, error) {
    g, err := fs.Load()
    if err != nil {
        return "", err
    }
    return g.Version, nil
}
```

---

## Testing Specification

### Test Cases and Coverage

```go
// internal/genesis/validator_test.go

package genesis

import (
    "testing"
    "time"
)

// TestValidateGenesisSchema_ValidGenesis tests happy path
func TestValidateGenesisSchema_ValidGenesis(t *testing.T) {
    v := NewSchemaValidator()
    
    g := &Genesis{
        Version: "1.0.0",
        ChainID: "x3-test-1",
        Timestamp: time.Now().UTC(),
        ConsensusParams: &ConsensusParams{
            BlockSize: &BlockSizeParams{
                MaxBytes: 1_000_000,
                MaxGas:   10_000_000,
            },
            EvidenceParams: &EvidenceParams{
                MaxAgeNumBlocks: 302_400,
                MaxAgeDuration:  30 * 24 * time.Hour,
            },
            ValidatorParams: &ValidatorParams{
                PubKeyTypes: []string{"ed25519"},
            },
        },
        Validators: []Validator{
            {
                Address:   "0000000000000000000000000000000000000001",
                PublicKey: "abcd1234" + "0000000000000000000000000000000000000000000000000000000000",
                Power:     100,
                Name:      "validator-1",
            },
        },
        Accounts: []Account{
            {
                Address: "1111111111111111111111111111111111111111",
                Balances: map[string]uint64{
                    "x3": 1_000_000_000,
                },
            },
        },
        AppState: make(map[string]interface{}),
    }
    
    errs := v.ValidateGenesisSchema(g)
    if len(errs) > 0 {
        t.Fatalf("Expected no errors, got %d: %v", len(errs), errs)
    }
}

// TestValidateGenesisSchema_MissingVersion tests validation catches missing version
func TestValidateGenesisSchema_MissingVersion(t *testing.T) {
    v := NewSchemaValidator()
    g := &Genesis{
        Version: "",
        ChainID: "x3-test-1",
    }
    
    errs := v.ValidateGenesisSchema(g)
    if len(errs) == 0 {
        t.Fatal("Expected validation error for missing version")
    }
    
    if errs[0].Constraint != "required" {
        t.Fatalf("Expected 'required' constraint, got %q", errs[0].Constraint)
    }
}

// TestValidateGenesisSchema_InvalidSemver tests semver validation
func TestValidateGenesisSchema_InvalidSemver(t *testing.T) {
    v := NewSchemaValidator()
    g := &Genesis{
        Version: "1.0",  // Missing patch
        ChainID: "x3-test-1",
    }
    
    errs := v.ValidateGenesisSchema(g)
    found := false
    for _, e := range errs {
        if e.Constraint == "semver" {
            found = true
            break
        }
    }
    if !found {
        t.Fatal("Expected semver validation error")
    }
}

// TestValidateGenesisSchema_InvalidValidatorAddress tests hex validation
func TestValidateGenesisSchema_InvalidValidatorAddress(t *testing.T) {
    v := NewSchemaValidator()
    g := &Genesis{
        Version: "1.0.0",
        ChainID: "x3-test-1",
        Timestamp: time.Now().UTC(),
        Validators: []Validator{
            {
                Address:   "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",  // Invalid hex
                PublicKey: "abcd1234" + "0000000000000000000000000000000000000000000000000000000000",
                Power:     100,
            },
        },
        AppState: make(map[string]interface{}),
    }
    
    errs := v.ValidateGenesisSchema(g)
    found := false
    for _, e := range errs {
        if e.Field == "Validators[0].Address" && e.Constraint == "hexadecimal" {
            found = true
            break
        }
    }
    if !found {
        t.Fatal("Expected hex validation error for address")
    }
}

// TestValidateGenesisSchema_DuplicateValidator tests uniqueness
func TestValidateGenesisSchema_DuplicateValidator(t *testing.T) {
    v := NewSchemaValidator()
    addr := "0000000000000000000000000000000000000001"
    g := &Genesis{
        Version: "1.0.0",
        ChainID: "x3-test-1",
        Timestamp: time.Now().UTC(),
        Validators: []Validator{
            {
                Address:   addr,
                PublicKey: "abcd1234" + "0000000000000000000000000000000000000000000000000000000000",
                Power:     100,
            },
            {
                Address:   addr,  // Duplicate
                PublicKey: "ef561234" + "0000000000000000000000000000000000000000000000000000000000",
                Power:     100,
            },
        },
        AppState: make(map[string]interface{}),
    }
    
    errs := v.ValidateGenesisSchema(g)
    found := false
    for _, e := range errs {
        if e.Constraint == "unique" {
            found = true
            break
        }
    }
    if !found {
        t.Fatal("Expected uniqueness validation error")
    }
}

// TestGenesisComputeHash tests hash determinism
func TestGenesisComputeHash_Deterministic(t *testing.T) {
    g := createTestGenesis()
    
    hash1, err := g.ComputeHash()
    if err != nil {
        t.Fatalf("compute hash 1: %v", err)
    }
    
    hash2, err := g.ComputeHash()
    if err != nil {
        t.Fatalf("compute hash 2: %v", err)
    }
    
    if hash1 != hash2 {
        t.Fatalf("hashes differ: %s vs %s", hash1, hash2)
    }
}

// TestGenesisComputeHash_ChangedContent tests hash changes when content changes
func TestGenesisComputeHash_ChangedContent(t *testing.T) {
    g1 := createTestGenesis()
    hash1, _ := g1.ComputeHash()
    
    g2 := createTestGenesis()
    g2.ChainID = "x3-test-2"
    hash2, _ := g2.ComputeHash()
    
    if hash1 == hash2 {
        t.Fatal("hashes should differ for different content")
    }
}

// Helper to create valid test genesis
func createTestGenesis() *Genesis {
    return &Genesis{
        Version: "1.0.0",
        ChainID: "x3-test-1",
        Timestamp: time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC),
        ConsensusParams: &ConsensusParams{
            BlockSize: &BlockSizeParams{
                MaxBytes: 1_000_000,
                MaxGas:   10_000_000,
            },
            EvidenceParams: &EvidenceParams{
                MaxAgeNumBlocks: 302_400,
                MaxAgeDuration:  30 * 24 * time.Hour,
            },
            ValidatorParams: &ValidatorParams{
                PubKeyTypes: []string{"ed25519"},
            },
        },
        Validators: []Validator{
            {
                Address:   "0000000000000000000000000000000000000001",
                PublicKey: "abcd1234" + "0000000000000000000000000000000000000000000000000000000000",
                Power:     100,
            },
        },
        Accounts: []Account{},
        AppState: make(map[string]interface{}),
    }
}
```

---

## Example Usage Patterns

### Basic Genesis Creation and Validation

```go
// Example: Create and validate genesis

func exampleCreateGenesis() error {
    g := &Genesis{
        Version: "1.0.0",
        ChainID: "x3-mainnet-1",
        Timestamp: time.Now().UTC(),
        ConsensusParams: &ConsensusParams{
            BlockSize: &BlockSizeParams{
                MaxBytes: 4_000_000,
                MaxGas:   40_000_000,
            },
            EvidenceParams: &EvidenceParams{
                MaxAgeNumBlocks: 302_400,
                MaxAgeDuration:  21 * 24 * time.Hour,
            },
            ValidatorParams: &ValidatorParams{
                PubKeyTypes: []string{"ed25519", "secp256k1"},
            },
        },
        Validators: []Validator{
            {
                Address:   "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
                PublicKey: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                Power:     1000,
                Name:      "validator-1",
            },
        },
        Accounts: []Account{
            {
                Address: "cafecafecafecafecafecafecafecafecafecafe",
                Balances: map[string]uint64{
                    "stake": 1_000_000_000_000,
                    "x3":    10_000_000_000_000,
                },
                Name: "treasury",
            },
        },
        AppState: map[string]interface{}{
            "bank": map[string]interface{}{
                "balances": []map[string]interface{}{},
            },
            "staking": map[string]interface{}{
                "unbondingTime": "504h0m0s",
            },
        },
    }
    
    // Validate
    validator := NewSchemaValidator()
    errs := validator.ValidateGenesisSchema(g)
    if len(errs) > 0 {
        return fmt.Errorf("validation failed: %v", errs)
    }
    
    // Save
    storage, _ := NewFileStorage("./genesis.json")
    return storage.Save(g)
}
```

---

## Acceptance Criteria Checklist

- [ ] **Types Defined**: Genesis, Validator, Account, ConsensusParams fully defined with validation tags
- [ ] **SchemaValidator**: Validates all constraints (semver, hex, uniqueness, power > 0)
- [ ] **Hash Function**: ComputeHash() returns deterministic SHA256
- [ ] **Migrations**: MigrationRegistry framework with at least one example migration (v1.0.0 → v1.1.0)
- [ ] **Storage**: FileStorage with atomic writes and error handling
- [ ] **Tests**: 95%+ coverage including edge cases (empty validators, invalid semver, duplicates, concurrent reads)
- [ ] **Race Detector**: All tests pass with `go test -race`
- [ ] **Documentation**: Complete Go doc comments on all exported types/functions
- [ ] **Examples**: Usage examples in genesis_test.go
- [ ] **Integration**: Ready for L0-02 Change Ledger audit trail recording

---

## File Structure Checklist

```
✓ internal/genesis/
  ├─ types.go (Genesis, Validator, Account, ConsensusParams)
  ├─ validator.go (SchemaValidator, validation logic)
  ├─ migration.go (MigrationRegistry, migration funcs)
  ├─ storage.go (FileStorage interface/impl)
  ├─ validator_test.go (validation tests)
  ├─ types_test.go (type/hash tests)
  ├─ migration_test.go (migration tests)
  └─ storage_test.go (file I/O tests)
✓ cmd/x3-genesis/
  └─ main.go (CLI: validate, migrate, load genesis)
✓ go.mod (dependencies: testify, standard lib only)
✓ Makefile (build, test, lint targets)
```

---

## Quality Gate Checklist

Before marking L0-01 complete:

- [ ] `go test ./internal/genesis/...` all passing
- [ ] `go test -race ./internal/genesis/...` all passing
- [ ] `go tool cover -html=coverage.out` shows >95% coverage
- [ ] `golangci-lint run ./internal/genesis/...` zero warnings
- [ ] `go doc ./internal/genesis/...` all exported symbols documented
- [ ] Manual testing: load valid genesis, catch validation errors, migrate versions
- [ ] File storage: atomic writes, concurrent reads, no partial writes
- [ ] PR review approved by protocol core team

---

## Next Steps (L0-02 Integration Point)

Once L0-01 is complete:

1. **L0-02 Change Ledger** will record all genesis mutations via audit trail
2. **L0-05 Local Chain Simulator** will load genesis and execute deterministically
3. **L1-01 Operator Registry** will validate operators against Genesis validators

**Start here. Execute. Deliver.** 🚀
