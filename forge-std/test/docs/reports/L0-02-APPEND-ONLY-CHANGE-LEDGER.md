# L0-02: Append-Only Change Ledger — Production Specification

**Task ID**: L0-02  
**Epic**: EPIC-1 (Core Protocol & Kernel)  
**Status**: Ready for Implementation  
**Estimated Duration**: 1 day  
**Prerequisite**: L0-01 (Genesis Configuration Engine)  
**Blocking**: L0-03, L0-04 (Operator Registry depends on audit trail)  
**Related Proposal**: X3-PROPOSAL-001

---

## Executive Summary

The Change Ledger is an immutable, append-only audit trail of all state mutations in X3. It:
- Records every change (create/update/delete) with full context
- Maintains cryptographic integrity via hash chaining
- Provides queryable audit trails for compliance and debugging
- Detects corruption via hash verification
- Integrates with all higher-level registries (Capability, Operator, Governance)

**Design Principle**: Write-once, query-many. No deletes, never overwrites.

---

## Domain Types

### Core Ledger Entities

```go
// internal/ledger/types.go

package ledger

import (
    "crypto/sha256"
    "encoding/hex"
    "encoding/json"
    "fmt"
    "time"
)

// ChangeEntry represents a single immutable change in the ledger.
type ChangeEntry struct {
    // ID is monotonically increasing entry identifier
    ID uint64 `json:"id"`
    
    // Timestamp is UTC creation time
    Timestamp time.Time `json:"timestamp" validate:"required"`
    
    // Operation is the type of change
    Operation ChangeOperation `json:"operation" validate:"required"`
    
    // EntityType identifies what was changed (genesis, capability, operator, governance)
    EntityType string `json:"entityType" validate:"required,min=3,max=32"`
    
    // EntityID identifies the specific entity
    EntityID string `json:"entityId" validate:"required,min=1,max=256"`
    
    // PreviousState is full state before change (nil for CREATE)
    PreviousState json.RawMessage `json:"previousState,omitempty"`
    
    // NewState is full state after change
    NewState json.RawMessage `json:"newState" validate:"required"`
    
    // ActorID identifies who made the change
    ActorID string `json:"actorId" validate:"required,min=1,max=256"`
    
    // Hash is SHA256(normalized entry) - prevents tampering
    Hash string `json:"hash" validate:"required,hexadecimal,len=64"`
    
    // PreviousHash is hash of entry N-1 (creates chain)
    PreviousHash string `json:"previousHash,omitempty" validate:"hexadecimal,len=64"`
    
    // Metadata for context (error codes, gas used, etc.)
    Metadata map[string]interface{} `json:"metadata,omitempty"`
}

// ChangeOperation is the type of change.
type ChangeOperation string

const (
    OperationCreate      ChangeOperation = "CREATE"
    OperationUpdate      ChangeOperation = "UPDATE"
    OperationDelete      ChangeOperation = "DELETE"
    OperationActivate    ChangeOperation = "ACTIVATE"
    OperationDeactivate  ChangeOperation = "DEACTIVATE"
    OperationSlash       ChangeOperation = "SLASH"
    OperationReward      ChangeOperation = "REWARD"
    OperationMigrate     ChangeOperation = "MIGRATE"
)

// ComputeHash returns SHA256 of normalized entry (excludes Hash field).
func (e *ChangeEntry) ComputeHash() (string, error) {
    // Create copy without hash fields for hashing
    copy := *e
    copy.Hash = ""
    copy.PreviousHash = ""
    
    normalized, err := json.Marshal(copy)
    if err != nil {
        return "", fmt.Errorf("marshal: %w", err)
    }
    
    hash := sha256.Sum256(normalized)
    return hex.EncodeToString(hash[:]), nil
}

// QueryFilter defines filtering criteria for ledger queries.
type QueryFilter struct {
    // StartID is minimum entry ID (inclusive)
    StartID uint64 `json:"startId,omitempty"`
    
    // EndID is maximum entry ID (inclusive)
    EndID uint64 `json:"endId,omitempty"`
    
    // EntityType filters by entity type
    EntityType string `json:"entityType,omitempty"`
    
    // EntityID filters by specific entity
    EntityID string `json:"entityId,omitempty"`
    
    // Operation filters by change type
    Operation ChangeOperation `json:"operation,omitempty"`
    
    // StartTime filters by time range (inclusive)
    StartTime *time.Time `json:"startTime,omitempty"`
    
    // EndTime filters by time range (inclusive)
    EndTime *time.Time `json:"endTime,omitempty"`
    
    // ActorID filters by who made change
    ActorID string `json:"actorId,omitempty"`
    
    // Limit maximum results (0 = all)
    Limit int `json:"limit,omitempty"`
}

// AuditTrail represents audit trail for a single entity.
type AuditTrail struct {
    // EntityID is the entity being audited
    EntityID string `json:"entityId"`
    
    // EntityType is type of entity
    EntityType string `json:"entityType"`
    
    // Entries are all changes to this entity in chronological order
    Entries []ChangeEntry `json:"entries"`
    
    // CurrentHash is hash of most recent entry
    CurrentHash string `json:"currentHash"`
    
    // IsValid indicates if entire chain validated
    IsValid bool `json:"isValid"`
}

// LedgerStats contains statistics about ledger state.
type LedgerStats struct {
    // TotalEntries is total count of entries
    TotalEntries uint64 `json:"totalEntries"`
    
    // LastEntryID is ID of most recent entry
    LastEntryID uint64 `json:"lastEntryID"`
    
    // LastTimestamp is time of most recent entry
    LastTimestamp time.Time `json:"lastTimestamp"`
    
    // LastHash is hash of most recent entry
    LastHash string `json:"lastHash"`
    
    // EntriesByType maps entity type to count
    EntriesByType map[string]uint64 `json:"entriesByType"`
    
    // EntriesByOperation maps operation to count
    EntriesByOperation map[string]uint64 `json:"entriesByOperation"`
}
```

---

## Writer Interface & Implementation

### Append-Only Write Operations

```go
// internal/ledger/writer.go

package ledger

import (
    "encoding/json"
    "fmt"
    "sync"
    "time"
)

// Writer appends new entries to ledger.
type Writer interface {
    // Append adds new entry to ledger
    Append(entry *ChangeEntry) error
    
    // GetLastHash returns hash of most recent entry
    GetLastHash() (string, error)
    
    // GetLastID returns ID of most recent entry
    GetLastID() (uint64, error)
}

// AppendOnlyWriter implements Writer with append-only semantics.
type AppendOnlyWriter struct {
    storage Storage
    mu      sync.RWMutex
    lastID  uint64
    lastHash string
}

// NewAppendOnlyWriter creates new writer with initial state.
func NewAppendOnlyWriter(storage Storage) (*AppendOnlyWriter, error) {
    w := &AppendOnlyWriter{
        storage: storage,
    }
    
    // Load existing state
    if err := w.loadState(); err != nil {
        return nil, err
    }
    
    return w, nil
}

// Append adds entry to ledger with validation and chaining.
func (w *AppendOnlyWriter) Append(entry *ChangeEntry) error {
    if entry == nil {
        return NewLedgerError("append", "entry is nil")
    }
    
    w.mu.Lock()
    defer w.mu.Unlock()
    
    // Validate entry
    if err := w.validateEntry(entry); err != nil {
        return err
    }
    
    // Set ID and hash chain
    w.lastID++
    entry.ID = w.lastID
    entry.PreviousHash = w.lastHash
    
    // Compute and set hash
    hash, err := entry.ComputeHash()
    if err != nil {
        return fmt.Errorf("compute hash: %w", err)
    }
    entry.Hash = hash
    
    // Atomically write to storage
    if err := w.storage.Write(entry); err != nil {
        return fmt.Errorf("storage write: %w", err)
    }
    
    // Update state
    w.lastHash = hash
    
    return nil
}

// GetLastHash returns hash of most recent entry.
func (w *AppendOnlyWriter) GetLastHash() (string, error) {
    w.mu.RLock()
    defer w.mu.RUnlock()
    return w.lastHash, nil
}

// GetLastID returns ID of most recent entry.
func (w *AppendOnlyWriter) GetLastID() (uint64, error) {
    w.mu.RLock()
    defer w.mu.RUnlock()
    return w.lastID, nil
}

// loadState initializes writer with existing ledger state.
func (w *AppendOnlyWriter) loadState() error {
    stats, err := w.storage.Stats()
    if err != nil {
        return fmt.Errorf("load stats: %w", err)
    }
    
    w.lastID = stats.LastEntryID
    w.lastHash = stats.LastHash
    
    return nil
}

// validateEntry checks entry for validity.
func (w *AppendOnlyWriter) validateEntry(entry *ChangeEntry) error {
    if entry.Timestamp.IsZero() {
        return NewLedgerError("timestamp", "timestamp required")
    }
    
    if entry.Operation == "" {
        return NewLedgerError("operation", "operation required")
    }
    
    if entry.EntityType == "" {
        return NewLedgerError("entityType", "entity type required")
    }
    
    if entry.EntityID == "" {
        return NewLedgerError("entityId", "entity ID required")
    }
    
    if entry.ActorID == "" {
        return NewLedgerError("actorId", "actor ID required")
    }
    
    if len(entry.NewState) == 0 {
        return NewLedgerError("newState", "new state required")
    }
    
    // Validate operations for create/update consistency
    if entry.Operation == OperationCreate && len(entry.PreviousState) > 0 {
        return NewLedgerError("operation", "CREATE should not have previous state")
    }
    
    return nil
}
```

---

## Reader Interface & Queryable Access

### Query Operations

```go
// internal/ledger/reader.go

package ledger

import (
    "fmt"
    "sort"
    "sync"
    "time"
)

// Reader queries entries from ledger.
type Reader interface {
    // QueryByID returns single entry by ID
    QueryByID(id uint64) (*ChangeEntry, error)
    
    // QueryByFilter returns entries matching filter
    QueryByFilter(filter QueryFilter) ([]ChangeEntry, error)
    
    // QueryAuditTrail returns all changes for entity
    QueryAuditTrail(entityType, entityID string) (*AuditTrail, error)
    
    // QueryTimeRange returns entries within time window
    QueryTimeRange(start, end time.Time) ([]ChangeEntry, error)
    
    // QueryEntityType returns all entries for entity type
    QueryEntityType(entityType string) ([]ChangeEntry, error)
    
    // GetStats returns ledger statistics
    GetStats() (*LedgerStats, error)
}

// AppendOnlyReader implements Reader with in-memory caching.
type AppendOnlyReader struct {
    storage Storage
    mu      sync.RWMutex
    cache   map[uint64]*ChangeEntry
}

// NewAppendOnlyReader creates new reader.
func NewAppendOnlyReader(storage Storage) *AppendOnlyReader {
    return &AppendOnlyReader{
        storage: storage,
        cache:   make(map[uint64]*ChangeEntry),
    }
}

// QueryByID returns single entry.
func (r *AppendOnlyReader) QueryByID(id uint64) (*ChangeEntry, error) {
    r.mu.RLock()
    if entry, ok := r.cache[id]; ok {
        r.mu.RUnlock()
        return entry, nil
    }
    r.mu.RUnlock()
    
    entry, err := r.storage.ReadByID(id)
    if err != nil {
        return nil, fmt.Errorf("read by ID: %w", err)
    }
    
    // Cache and return
    r.mu.Lock()
    r.cache[id] = entry
    r.mu.Unlock()
    
    return entry, nil
}

// QueryByFilter returns entries matching filter.
func (r *AppendOnlyReader) QueryByFilter(filter QueryFilter) ([]ChangeEntry, error) {
    entries, err := r.storage.ReadByFilter(filter)
    if err != nil {
        return nil, fmt.Errorf("read by filter: %w", err)
    }
    return entries, nil
}

// QueryAuditTrail returns all changes for single entity.
func (r *AppendOnlyReader) QueryAuditTrail(entityType, entityID string) (*AuditTrail, error) {
    filter := QueryFilter{
        EntityType: entityType,
        EntityID:   entityID,
    }
    
    entries, err := r.storage.ReadByFilter(filter)
    if err != nil {
        return nil, fmt.Errorf("read audit trail: %w", err)
    }
    
    // Sort by ID (should already be ordered, but ensure it)
    sort.Slice(entries, func(i, j int) bool {
        return entries[i].ID < entries[j].ID
    })
    
    // Validate hash chain
    trail := &AuditTrail{
        EntityType: entityType,
        EntityID:   entityID,
        Entries:    entries,
        IsValid:    true,
    }
    
    if err := r.validateChain(entries); err != nil {
        trail.IsValid = false
        return trail, nil // Return even if invalid
    }
    
    if len(entries) > 0 {
        trail.CurrentHash = entries[len(entries)-1].Hash
    }
    
    return trail, nil
}

// QueryTimeRange returns entries within time range.
func (r *AppendOnlyReader) QueryTimeRange(start, end time.Time) ([]ChangeEntry, error) {
    filter := QueryFilter{
        StartTime: &start,
        EndTime:   &end,
    }
    return r.QueryByFilter(filter)
}

// QueryEntityType returns all entries for entity type.
func (r *AppendOnlyReader) QueryEntityType(entityType string) ([]ChangeEntry, error) {
    filter := QueryFilter{
        EntityType: entityType,
    }
    return r.QueryByFilter(filter)
}

// GetStats returns ledger statistics.
func (r *AppendOnlyReader) GetStats() (*LedgerStats, error) {
    return r.storage.Stats()
}

// validateChain verifies hash chain integrity.
func (r *AppendOnlyReader) validateChain(entries []ChangeEntry) error {
    var prevHash string
    
    for i, entry := range entries {
        // Recompute hash
        computedHash, err := entry.ComputeHash()
        if err != nil {
            return fmt.Errorf("compute hash for entry %d: %w", i, err)
        }
        
        // Compare with stored hash
        if computedHash != entry.Hash {
            return fmt.Errorf("entry %d hash mismatch: expected %s, got %s",
                i, computedHash, entry.Hash)
        }
        
        // Check chain link (except first entry)
        if i > 0 && entry.PreviousHash != prevHash {
            return fmt.Errorf("entry %d chain broken: expected %s, got %s",
                i, prevHash, entry.PreviousHash)
        }
        
        prevHash = entry.Hash
    }
    
    return nil
}
```

---

## Storage Abstraction

### File-Based Storage (SQLite WAL variant)

```go
// internal/ledger/storage.go

package ledger

import (
    "database/sql"
    "encoding/json"
    "fmt"
    "sync"
    "time"
    
    _ "github.com/mattn/go-sqlite3"
)

// Storage interface abstracts ledger persistence.
type Storage interface {
    // Write appends entry to ledger
    Write(entry *ChangeEntry) error
    
    // ReadByID returns single entry
    ReadByID(id uint64) (*ChangeEntry, error)
    
    // ReadByFilter returns entries matching filter
    ReadByFilter(filter QueryFilter) ([]ChangeEntry, error)
    
    // Stats returns ledger statistics
    Stats() (*LedgerStats, error)
    
    // Close closes storage
    Close() error
}

// SQLiteStorage uses SQLite with WAL mode for durability.
type SQLiteStorage struct {
    db *sql.DB
    mu sync.RWMutex
}

// NewSQLiteStorage creates SQLite-backed storage.
func NewSQLiteStorage(dsn string) (*SQLiteStorage, error) {
    db, err := sql.Open("sqlite3", dsn)
    if err != nil {
        return nil, fmt.Errorf("open database: %w", err)
    }
    
    // Enable WAL for durability
    if _, err := db.Exec("PRAGMA journal_mode = WAL"); err != nil {
        return nil, fmt.Errorf("enable WAL: %w", err)
    }
    
    // Create table if not exists
    if err := createLedgerTable(db); err != nil {
        return nil, fmt.Errorf("create table: %w", err)
    }
    
    return &SQLiteStorage{db: db}, nil
}

// Write appends entry atomically.
func (s *SQLiteStorage) Write(entry *ChangeEntry) error {
    s.mu.Lock()
    defer s.mu.Unlock()
    
    previousState := []byte(nil)
    if entry.PreviousState != nil {
        previousState = entry.PreviousState
    }
    
    metadata := []byte("{}")
    if entry.Metadata != nil {
        data, _ := json.Marshal(entry.Metadata)
        metadata = data
    }
    
    stmt := `
        INSERT INTO ledger (
            id, timestamp, operation, entity_type, entity_id,
            previous_state, new_state, actor_id, hash, previous_hash, metadata
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `
    
    _, err := s.db.Exec(
        stmt,
        entry.ID, entry.Timestamp, entry.Operation, entry.EntityType, entry.EntityID,
        previousState, entry.NewState, entry.ActorID, entry.Hash, entry.PreviousHash,
        metadata,
    )
    
    return err
}

// ReadByID returns single entry.
func (s *SQLiteStorage) ReadByID(id uint64) (*ChangeEntry, error) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    stmt := "SELECT * FROM ledger WHERE id = ?"
    row := s.db.QueryRow(stmt, id)
    
    return scanEntry(row)
}

// ReadByFilter returns entries matching filter.
func (s *SQLiteStorage) ReadByFilter(filter QueryFilter) ([]ChangeEntry, error) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    query := "SELECT * FROM ledger WHERE 1=1"
    args := []interface{}{}
    
    if filter.StartID > 0 {
        query += " AND id >= ?"
        args = append(args, filter.StartID)
    }
    if filter.EndID > 0 {
        query += " AND id <= ?"
        args = append(args, filter.EndID)
    }
    if filter.EntityType != "" {
        query += " AND entity_type = ?"
        args = append(args, filter.EntityType)
    }
    if filter.EntityID != "" {
        query += " AND entity_id = ?"
        args = append(args, filter.EntityID)
    }
    if filter.Operation != "" {
        query += " AND operation = ?"
        args = append(args, filter.Operation)
    }
    if filter.StartTime != nil {
        query += " AND timestamp >= ?"
        args = append(args, filter.StartTime)
    }
    if filter.EndTime != nil {
        query += " AND timestamp <= ?"
        args = append(args, filter.EndTime)
    }
    if filter.ActorID != "" {
        query += " AND actor_id = ?"
        args = append(args, filter.ActorID)
    }
    
    query += " ORDER BY id ASC"
    
    if filter.Limit > 0 {
        query += " LIMIT ?"
        args = append(args, filter.Limit)
    }
    
    rows, err := s.db.Query(query, args...)
    if err != nil {
        return nil, fmt.Errorf("query: %w", err)
    }
    defer rows.Close()
    
    var entries []ChangeEntry
    for rows.Next() {
        entry, err := scanEntryFromRows(rows)
        if err != nil {
            return nil, err
        }
        entries = append(entries, *entry)
    }
    
    return entries, rows.Err()
}

// Stats returns ledger statistics.
func (s *SQLiteStorage) Stats() (*LedgerStats, error) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    stats := &LedgerStats{
        EntriesByType:      make(map[string]uint64),
        EntriesByOperation: make(map[string]uint64),
    }
    
    // Get total count and last entry
    row := s.db.QueryRow(`
        SELECT COUNT(*), MAX(id), MAX(timestamp), MAX(hash) FROM ledger
    `)
    if err := row.Scan(&stats.TotalEntries, &stats.LastEntryID, &stats.LastTimestamp, &stats.LastHash); err != nil {
        return nil, err
    }
    
    // Get counts by type
    rows, err := s.db.Query(`
        SELECT entity_type, COUNT(*) FROM ledger GROUP BY entity_type
    `)
    if err == nil {
        defer rows.Close()
        for rows.Next() {
            var et string
            var count uint64
            rows.Scan(&et, &count)
            stats.EntriesByType[et] = count
        }
    }
    
    // Get counts by operation
    rows, err = s.db.Query(`
        SELECT operation, COUNT(*) FROM ledger GROUP BY operation
    `)
    if err == nil {
        defer rows.Close()
        for rows.Next() {
            var op string
            var count uint64
            rows.Scan(&op, &count)
            stats.EntriesByOperation[op] = count
        }
    }
    
    return stats, nil
}

// Close closes database connection.
func (s *SQLiteStorage) Close() error {
    return s.db.Close()
}

// Helper functions

func createLedgerTable(db *sql.DB) error {
    _, err := db.Exec(`
        CREATE TABLE IF NOT EXISTS ledger (
            id INTEGER PRIMARY KEY,
            timestamp DATETIME NOT NULL,
            operation TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            previous_state BLOB,
            new_state BLOB NOT NULL,
            actor_id TEXT NOT NULL,
            hash TEXT NOT NULL UNIQUE,
            previous_hash TEXT,
            metadata JSON,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_entity ON ledger(entity_type, entity_id);
        CREATE INDEX IF NOT EXISTS idx_timestamp ON ledger(timestamp);
        CREATE INDEX IF NOT EXISTS idx_operation ON ledger(operation);
        CREATE INDEX IF NOT EXISTS idx_actor ON ledger(actor_id);
    `)
    return err
}

func scanEntry(row *sql.Row) (*ChangeEntry, error) {
    var e ChangeEntry
    var previousState sql.NullString
    var metadata sql.NullString
    var createdAt time.Time
    
    err := row.Scan(
        &e.ID, &e.Timestamp, &e.Operation, &e.EntityType, &e.EntityID,
        &previousState, &e.NewState, &e.ActorID, &e.Hash, &e.PreviousHash,
        &metadata, &createdAt,
    )
    
    if previousState.Valid {
        e.PreviousState = []byte(previousState.String)
    }
    if metadata.Valid {
        json.Unmarshal([]byte(metadata.String), &e.Metadata)
    }
    
    return &e, err
}

func scanEntryFromRows(rows *sql.Rows) (*ChangeEntry, error) {
    var e ChangeEntry
    var previousState sql.NullString
    var metadata sql.NullString
    var createdAt time.Time
    
    err := rows.Scan(
        &e.ID, &e.Timestamp, &e.Operation, &e.EntityType, &e.EntityID,
        &previousState, &e.NewState, &e.ActorID, &e.Hash, &e.PreviousHash,
        &metadata, &createdAt,
    )
    
    if previousState.Valid {
        e.PreviousState = []byte(previousState.String)
    }
    if metadata.Valid {
        json.Unmarshal([]byte(metadata.String), &e.Metadata)
    }
    
    return &e, err
}
```

---

## Error Handling

### Ledger Error Types

```go
// internal/ledger/errors.go

package ledger

import "fmt"

// LedgerError represents ledger-specific errors.
type LedgerError struct {
    Code    string
    Field   string
    Message string
}

func (e LedgerError) Error() string {
    if e.Field != "" {
        return fmt.Sprintf("[%s] %s: %s", e.Code, e.Field, e.Message)
    }
    return fmt.Sprintf("[%s] %s", e.Code, e.Message)
}

func NewLedgerError(field string, msg string) LedgerError {
    return LedgerError{
        Code:    "LEDGER_ERROR",
        Field:   field,
        Message: msg,
    }
}
```

---

## Testing Specification

```go
// internal/ledger/writer_test.go

package ledger

import (
    "encoding/json"
    "testing"
    "time"
)

// TestAppendEntry tests basic append operation
func TestAppendEntry(t *testing.T) {
    storage := NewMemoryStorage()
    defer storage.Close()
    
    writer, _ := NewAppendOnlyWriter(storage)
    
    entry := &ChangeEntry{
        Timestamp:  time.Now().UTC(),
        Operation:  OperationCreate,
        EntityType: "capability",
        EntityID:   "cap-001",
        NewState:   json.RawMessage(`{"id":"cap-001","name":"read"}`),
        ActorID:    "genesis",
    }
    
    err := writer.Append(entry)
    if err != nil {
        t.Fatalf("append failed: %v", err)
    }
    
    if entry.ID != 1 {
        t.Fatalf("expected ID=1, got %d", entry.ID)
    }
    
    if entry.Hash == "" {
        t.Fatal("hash not computed")
    }
}

// TestConcurrentWrites tests thread safety
func TestConcurrentWrites(t *testing.T) {
    storage := NewMemoryStorage()
    defer storage.Close()
    
    writer, _ := NewAppendOnlyWriter(storage)
    
    // Write 100 entries concurrently
    done := make(chan error, 100)
    for i := 0; i < 100; i++ {
        go func(idx int) {
            entry := &ChangeEntry{
                Timestamp:  time.Now().UTC(),
                Operation:  OperationCreate,
                EntityType: "test",
                EntityID:   fmt.Sprintf("entity-%d", idx),
                NewState:   json.RawMessage("{}"),
                ActorID:    "test",
            }
            done <- writer.Append(entry)
        }(i)
    }
    
    for i := 0; i < 100; i++ {
        if err := <-done; err != nil {
            t.Fatalf("concurrent write failed: %v", err)
        }
    }
    
    // Verify all entries written
    lastID, _ := writer.GetLastID()
    if lastID != 100 {
        t.Fatalf("expected 100 entries, got %d", lastID)
    }
}

// TestHashChainIntegrity tests hash chain validation
func TestHashChainIntegrity(t *testing.T) {
    storage := NewMemoryStorage()
    writer, _ := NewAppendOnlyWriter(storage)
    reader := NewAppendOnlyReader(storage)
    
    // Write multiple entries
    for i := 0; i < 10; i++ {
        entry := &ChangeEntry{
            Timestamp:  time.Now().UTC(),
            Operation:  OperationCreate,
            EntityType: "entity",
            EntityID:   "test",
            NewState:   json.RawMessage("{}"),
            ActorID:    "test",
        }
        writer.Append(entry)
    }
    
    // Query and validate chain
    trail, _ := reader.QueryAuditTrail("entity", "test")
    if !trail.IsValid {
        t.Fatal("chain validation failed")
    }
}

// TestQueryByFilter tests filtering
func TestQueryByFilter(t *testing.T) {
    storage := NewMemoryStorage()
    writer, _ := NewAppendOnlyWriter(storage)
    reader := NewAppendOnlyReader(storage)
    
    // Write entries with different operations
    for _, op := range []ChangeOperation{OperationCreate, OperationUpdate, OperationCreate} {
        entry := &ChangeEntry{
            Timestamp:  time.Now().UTC(),
            Operation:  op,
            EntityType: "test",
            EntityID:   "entity-1",
            NewState:   json.RawMessage("{}"),
            ActorID:    "test",
        }
        writer.Append(entry)
    }
    
    // Filter by operation
    filter := QueryFilter{Operation: OperationCreate}
    entries, _ := reader.QueryByFilter(filter)
    
    if len(entries) != 2 {
        t.Fatalf("expected 2 CREATE operations, got %d", len(entries))
    }
}
```

---

## Acceptance Criteria

- [ ] ChangeEntry type with all required fields (ID, Timestamp, Operation, Hash, Chain)
- [ ] AppendOnlyWriter with atomic append, hash computation, chain linking
- [ ] Hash chain validation preventing tampering
- [ ] Reader with QueryByID, QueryByFilter, QueryAuditTrail, QueryTimeRange
- [ ] SQLiteStorage with WAL mode for durability
- [ ] 95%+ test coverage including concurrent writes (race detector)
- [ ] Performance: >1000 writes/sec, <100ms query response
- [ ] All tests passing with race detector
- [ ] Complete Go doc comments
- [ ] Integration with L0-01 Genesis (audit events)

---

## Quality Gates

- [ ] `go test -race ./internal/ledger/...` passes
- [ ] `go tool cover` >95% coverage
- [ ] Concurrent write safety verified
- [ ] Hash chain integrity verified
- [ ] Query performance benchmarks documented
- [ ] Integration test: Genesis → Ledger persistence flow

**Start coding. Execute.** 🚀
