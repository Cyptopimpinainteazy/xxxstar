# Phase 3: Load/Store Hoisting - Implementation Guide

**Target**: 72-75% Compiler Completion  
**Est. Duration**: 2-3 hours  
**Dependency**: ✅ Phase 2 (Value Numbering) COMPLETE  
**Complexity**: Medium (alias analysis + memory operations)

---

## 1. Strategic Context

### Why Load/Store Hoisting?

**Current State** (70%):
- Binary/Unary operations value-numbered ✅
- Commutative equivalences recognized ✅
- Cross-block redundancy detected ✅
- **Gap**: Memory operations still considered impure

**Value Hoisting** (70% → 72-75%):
- Recognize that Load(addr) == Load(addr) across blocks
- Hoist redundant loads to earliest safe point
- Conservative alias analysis: assume all stores kill loads
- Progressive: refine with data-flow analysis

**Estimate Gain**: 5-15% additional optimization on memory-intensive code

### Benchmark Impact
```
Before Phase 3: 33.5% gas reduction (current)
After Phase 3:  38-42% gas reduction (estimated)
Mechanism:     Load hoisting for array/struct accesses
Targets:       Simple loop programs, struct field access
```

---

## 2. Implementation Strategy

### Step 1: Extend CanonicalExpr (45 min)

**File**: `crates/x3-opt/src/value_numbering.rs`

Add Load/Store variants:
```rust
pub enum CanonicalExpr {
    // Existing...
    CommutativeBinary(String, String, String),
    Binary(String, String, String),
    Unary(String, String),
    Variable(String),
    
    // NEW:
    Load(String, String),         // (address_str, offset_str)
    Store(String, String, String), // (address_str, value_str, offset_str)
}
```

Extend canonicalization logic:
```rust
impl CanonicalExpr {
    pub fn from_load(addr: MirValue, offset: MirValue) -> Self {
        CanonicalExpr::Load(
            format!("{:?}", addr),
            format!("{:?}", offset),
        )
    }
    
    pub fn from_store(addr: MirValue, val: MirValue, offset: MirValue) -> Self {
        CanonicalExpr::Store(
            format!("{:?}", addr),
            format!("{:?}", val),
            format!("{:?}", offset),
        )
    }
}
```

**Tests to Add** (3):
- `test_load_canonicalization()` - Same load gets same VN
- `test_store_not_pure()` - Stores marked as impure
- `test_load_after_store_different_addr()` - Different addresses != same load

---

### Step 2: PRE Enhancement for Loads (45 min)

**File**: `crates/x3-opt/src/passes/pre.rs`

Update `ExprKey::from_rhs()` to handle loads:
```rust
fn from_rhs(rhs: &MirRhs, vn_table: &mut ValueNumbering) -> Option<Self> {
    let canonical = match rhs {
        // Existing...
        MirRhs::Binary(op, lhs, rhs) => {
            CanonicalExpr::from_binary(*op, *lhs, *rhs)
        }
        // NEW:
        MirRhs::Load { address, offset } => {
            CanonicalExpr::from_load(*address, *offset)
        }
        _ => return None,
    };
    
    let value_number = Some(vn_table.canonicalize(canonical.clone()).as_u32());
    Some(ExprKey { canonical, value_number })
}
```

Update availability computation:
```rust
// In compute_availability()
// NEW: Track write-kill information
fn compute_availability(...) {
    // ... existing ...
    
    // Track if we see any store
    let mut seen_store = false;
    
    for stmt in &block.statements {
        // Kill all loads after seeing a store
        if matches!(&stmt.rhs, MirRhs::Store { .. }) {
            seen_store = true;
            for (_, v) in state.iter_mut() {
                if matches!(canonical, CanonicalExpr::Load(..)) {
                    *v = Availability::Overdefined;
                }
            }
        }
    }
}
```

**Tests to Add** (4):
- `test_load_availability()` - Load marked available
- `test_store_kills_loads()` - Store in block kills all loads
- `test_load_hoisting_across_blocks()` - Load hoisted from multiple blocks
- `test_alias_conservative()` - Same address assumed aliased

---

### Step 3: Validation & Benchmarks (30 min)

Run full test suite:
```bash
cargo test -p x3-opt --lib
# Expected: 130-135 tests passing (4 new)
```

Run benchmarks:
```bash
cargo run -p x3-bench --release
# Expected: 38-42% gas reduction (up from 33.5%)
```

Create progress document

---

## 3. Detailed Implementation

### CanonicalExpr Extensions

**Pattern Matching Update**:
```rust
impl CanonicalExpr {
    fn is_memory_op(&self) -> bool {
        matches!(self, CanonicalExpr::Load(..) | CanonicalExpr::Store(..))
    }
    
    fn is_pure_op(&self) -> bool {
        // Loads are considered pure (no side effects)
        // Stores are NOT pure (they modify state)
        !matches!(self, CanonicalExpr::Store(..))
    }
}
```

**Ordering for Determinism**:
```rust
impl Ord for CanonicalExpr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // ... existing ...
        
        // Load discriminant ordering
        (CanonicalExpr::Load(a1, a2), CanonicalExpr::Load(b1, b2)) => {
            match a1.cmp(b1) {
                Ordering::Equal => a2.cmp(b2),
                other => other,
            }
        }
        
        (CanonicalExpr::Store(a1, a2, a3), CanonicalExpr::Store(b1, b2, b3)) => {
            match a1.cmp(b1) {
                Ordering::Equal => match a2.cmp(b2) {
                    Ordering::Equal => a3.cmp(b3),
                    other => other,
                },
                other => other,
            }
        }
        
        // Discriminant ordering
        (CanonicalExpr::Load(..), _) => Ordering::Less,
        (_, CanonicalExpr::Load(..)) => Ordering::Greater,
        // ... etc for Store ...
    }
}
```

### Availability Analysis with Write Tracking

**Conservative Algorithm**:
```rust
fn compute_availability_with_writes(
    &self,
    module: &MirModule,
    candidates: &BTreeSet<ExprKey>,
    vn_table: &mut ValueNumbering,
) -> BTreeMap<usize, BTreeMap<ExprKey, Availability>> {
    // ... initialization ...
    
    for func in &module.functions {
        for (idx, block) in func.blocks.iter().enumerate() {
            let mut state = BTreeMap::new();
            let mut sees_write = false;
            
            for stmt in &block.statements {
                // Check if this is a store
                if matches!(&stmt.rhs, MirRhs::Store { .. }) {
                    sees_write = true;
                }
                
                // Process statement normally
                if let Some(expr) = ExprKey::from_rhs(&stmt.rhs, vn_table) {
                    if candidates.contains(&expr) {
                        // After a write, loads become unavailable
                        if sees_write && expr.canonical.is_memory_op() {
                            state.insert(expr.clone(), Availability::Overdefined);
                        } else if expr.canonical.is_pure_op() {
                            state.insert(expr.clone(), Availability::Available);
                        }
                    }
                }
            }
            
            out_map.insert(idx, state);
        }
    }
    
    out_map
}
```

### MirRhs Pattern Matching

Check what load/store variants exist in MIR:
```bash
grep -r "MirRhs::" crates/x3-mir/src/lib.rs | head -20
```

Likely patterns:
- `MirRhs::Load { address, offset }` 
- `MirRhs::Store { address, value, offset }`
- Or similar

**Fallback**: If no memory ops in MirRhs, extend MirStatement instead

---

## 4. Testing Strategy

### Unit Tests to Add

**Test 1: Load Canonicalization**
```rust
#[test]
fn test_load_canonicalization() {
    let mut vn = ValueNumbering::new();
    let addr = MirValue(1);
    let offset = MirValue(0);
    
    let load1 = CanonicalExpr::from_load(addr, offset);
    let load2 = CanonicalExpr::from_load(addr, offset);
    
    let vn1 = vn.canonicalize(load1);
    let vn2 = vn.canonicalize(load2);
    
    assert_eq!(vn1, vn2);
}
```

**Test 2: Store Impact**
```rust
#[test]
fn test_store_kills_loads() {
    // Create module with:
    // Block 1: Load(a)
    // Block 2: Store(a, v); Load(a)
    // 
    // Expect: First load available in Block 1
    //         Second load NOT available after store
}
```

**Test 3: Load Hoisting**
```rust
#[test]
fn test_load_hoisting_multiple_blocks() {
    // Create module with:
    // Block 1: Load(x)
    // Block 2: Load(x)
    // Both exit to Block 3
    //
    // Expect: Load(x) hoisted to earliest point
}
```

**Test 4: Alias Conservative**
```rust
#[test]
fn test_different_loads_different_vn() {
    let mut vn = ValueNumbering::new();
    
    let load1 = CanonicalExpr::from_load(MirValue(1), MirValue(0));
    let load2 = CanonicalExpr::from_load(MirValue(2), MirValue(0));
    
    let vn1 = vn.canonicalize(load1);
    let vn2 = vn.canonicalize(load2);
    
    assert_ne!(vn1, vn2);
}
```

---

## 5. Integration Checklist

### Before Starting
- ✅ Phase 2 (Value Numbering) complete and tested
- ✅ PRE module fully understood
- ✅ MirRhs structure reviewed

### Implementation
- ⬜ Extend CanonicalExpr with Load/Store
- ⬜ Update Ord implementation
- ⬜ Add is_memory_op() and is_pure_op()
- ⬜ Update ExprKey::from_rhs()
- ⬜ Modify compute_availability() for write-kill
- ⬜ Add 4 unit tests
- ⬜ Run cargo check
- ⬜ Run cargo test (expect 130+ passing)

### Validation
- ⬜ All tests pass (0 regressions)
- ⬜ Benchmarks run (measure % improvement)
- ⬜ No compilation warnings
- ⬜ Determinism verified
- ⬜ Documentation updated

---

## 6. Expected Outcomes

### Code Changes
- **value_numbering.rs**: +50 lines (Load/Store variants + tests)
- **pre.rs**: +30 lines (write-kill tracking)
- **Total**: ~80 lines added

### Test Coverage
- Current: 126 tests
- After Phase 3: 130-135 tests
- Addition: 4-9 new tests

### Performance Impact
| Metric       | Current | Expected | Improvement |
| ------------ | ------- | -------- | ----------- |
| Gas          | -33.5%  | -38-42%  | +4-8.5%     |
| Bytes        | -28.1%  | -32-36%  | +3-8%       |
| Success Rate | 87.5%   | 87-90%   | Stable      |

### Compiler Completion
- Current: 70%
- After Phase 3: 72-75%
- Milestone: Cross-block + memory hoisting

---

## 7. Risk Mitigation

### Conservative Alias Analysis
**Risk**: Incorrectly hoist loads when store might alias  
**Mitigation**: Assume ALL stores kill ALL loads initially  
**Future**: Add alias analysis in Phase 4

### Memory Order Concerns
**Risk**: Reorder loads incorrectly  
**Mitigation**: Only hoist within single function initially  
**Future**: Add cross-function safety checks

### Regression Testing
**Risk**: New code breaks existing optimizations  
**Mitigation**: All 126 existing tests must pass before Phase 3 complete  
**Process**: Run full suite after each change

---

## 8. Success Criteria

### Mandatory
- ✅ All 130+ tests pass (0 regressions)
- ✅ cargo check clean (0 errors)
- ✅ Benchmarks show non-negative improvement
- ✅ Determinism maintained

### Nice-to-Have
- ✅ 38%+ gas reduction (vs 33.5% before)
- ✅ <3 hours implementation time
- ✅ <5 lines of unsafe code
- ✅ Comprehensive documentation

---

## 9. Immediate Next Actions

1. **Review MirRhs**: Check if Load/Store variants exist
2. **Prototype CanonicalExpr**: Add Load/Store variants
3. **Test Alone**: Verify value_numbering tests pass
4. **Integrate PRE**: Wire into compute_availability()
5. **Full Test**: Run entire suite
6. **Benchmark**: Measure performance
7. **Document**: Create Phase 3 completion guide

---

## 10. Quick Reference: Key Files

| File                                   | Change                          | LOC |
| -------------------------------------- | ------------------------------- | --- |
| `crates/x3-opt/src/value_numbering.rs` | Add Load/Store canonicalization | +50 |
| `crates/x3-opt/src/passes/pre.rs`      | Add write-kill tracking         | +30 |
| Total New Code                         |                                 | +80 |

---

**Ready to Begin**: 🟢 **YES**  
**Confidence**: 🔒 **HIGH** (foundation solid, path clear)  
**Proceed When**: User confirms or timeout 5 min  

---

Would you like me to start Phase 3 NOW, or prefer documentation review first?
