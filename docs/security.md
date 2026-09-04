# X3 Chain Security Model

This document outlines the comprehensive security architecture of X3 Chain's dual-VM blockchain platform, covering threat models, mitigation strategies, and best practices for developers.

## Security Overview

X3 Chain implements a multi-layered security model that combines proven security patterns from both EVM and SVM ecosystems while adding new protections for cross-VM operations.

```
┌─────────────────────────────────────────────────────────────────┐
│                    SECURITY LAYERS                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────────┐ │
│  │ NETWORK     │ │ CONSENSUS   │ │ EXECUTION                   │ │
│  │ SECURITY    │ │ SECURITY    │ │ SECURITY                    │ │
│  │             │ │             │ │                             │ │
│  │ • TLS       │ │ • Aura      │ │ • VM Isolation              │ │
│  │ • libp2p    │ │ • GRANDPA   │ │ • Atomic Execution          │ │
│  │ • Peer Auth │ │ • BABE      │ │ • Gas Metering              │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────────┘ │
│           │                   │                                 │
│           └───────────────────┼─────────────────────────────────┘
│                               │                                 │
│  ┌─────────────────────────────▼─────────────────────────────┐ │
│  │                  CROSS-VM SECURITY                       │ │
│  │                                                         │ │
│  │ • Call Validation    • State Synchronization            │ │
│  │ • ABI Encoding       • Atomic Commit                    │ │
│  │ • Reentrancy Guard   • Resource Limits                  │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters**: Layered security approach ensures that even if one layer is compromised, other layers provide defense in depth.

## Threat Model

### Adversary Capabilities

X3 Chain's threat model considers sophisticated adversaries with the following capabilities:

| Capability | Description | Mitigation |
|------------|-------------|------------|
| **Network-level** | Node-to-node communication interception | TLS encryption, peer authentication |
| **Consensus manipulation** | Validator collusion, eclipse attacks | Aura + GRANDPA finality, economic incentives |
| **Smart contract exploitation** | Reentrancy, integer overflow, MEV | Battle-tested VM semantics, static analysis |
| **Cross-VM attacks** | State desynchronization, call manipulation | Atomic execution, validation protocols |
| **Resource exhaustion** | Gas pumping, infinite loops | Gas metering, compute unit limits |

**Why this matters**: Understanding adversary capabilities helps developers design robust applications that can withstand sophisticated attacks.

### Attack Vectors

#### 1. Cross-VM State Desynchronization
**Threat**: EVM and SVM states become inconsistent during cross-VM execution.

**Scenario**:
```
1. EVM contract updates user balance: 1000 -> 500 X3
2. SVM program fails during execution
3. SVM state shows 1000 X3 (should be 500)
```

**Mitigation**:
- Atomic execution ensures both succeed or both revert
- Canonical ledger validates state consistency
- Cross-VM bridge performs consistency checks

**Why this matters**: Without atomicity, cross-VM operations could create inconsistent states that break application logic.

#### 2. Reentrancy via Cross-VM Calls
**Threat**: Cross-VM calls create new reentrancy vectors not present in single-VM systems.

**Scenario**:
```
1. EVM contract calls SVM program
2. SVM program calls back to EVM contract (different function)
3. EVM state may be in inconsistent intermediate state
```

**Mitigation**:
- Cross-VM reentrancy guards track call depth
- Maximum call depth limits (64 levels)
- State validation before and after cross-VM calls

**Why this matters**: Cross-VM reentrancy can bypass traditional single-VM protections, creating new attack surfaces.

#### 3. Gas/Compute Unit Manipulation
**Threat**: Attackers manipulate gas or compute units to cause DoS or inconsistent execution.

**Scenario**:
```
1. Attacker submits cross-VM transaction
2. EVM part consumes all gas
3. SVM part never executes or executes with limited resources
```

**Mitigation**:
- Separate gas limits for EVM and SVM components
- Pre-execution resource estimation
- Fail-fast if resources insufficient

**Why this matters**: Resource manipulation could enable attacks where one VM succeeds while the other fails, breaking atomicity.

## VM Security Isolation

### EVM Security Guarantees

X3 Chain maintains full EVM compatibility and security:

**Opcode Semantics**: All Ethereum opcodes behave identically to mainnet Ethereum
**Gas Model**: Standard Ethereum gas accounting with X3 Chain optimizations
**Memory Model**: Same memory and storage semantics as Ethereum
**Call Stack**: Standard EVM call stack with depth limits

**Security Features**:
- Protection against integer overflow/underflow
- Deterministic execution across all nodes
- Mature static analysis tooling compatibility
- Battle-tested security patterns

**Why this matters**: EVM compatibility ensures existing security tools and practices continue to work.

### SVM Security Guarantees

X3 Chain maintains full SVM compatibility and security:

**BPF Execution**: Solana BPF bytecode execution with rBPF
**Account Model**: Solana's account-based security model
**Parallel Execution**: Sealevel runtime isolation
**Compute Units**: Solana-style compute unit metering

**Security Features**:
- Account ownership verification
- Program-derived address (PDA) security
- Parallel execution isolation
- Solana runtime protections

**Why this matters**: SVM compatibility ensures existing Solana security model and tools continue to work.

### Cross-VM Security Bridge

The cross-VM bridge implements additional security layers:

```solidity
contract CrossVmSecurity {
    struct ExecutionContext {
        uint256 gasLimit;
        uint256 computeUnitLimit;
        bytes32 callHash;
        uint256 timestamp;
        address caller;
    }
    
    mapping(bytes32 => ExecutionContext) public contexts;
    
    modifier crossVmGuard() {
        bytes32 ctxHash = keccak256(abi.encode(
            msg.sender,
            gasleft(),
            block.timestamp,
            tx.origin
        ));
        
        require(contexts[ctxHash].timestamp == 0, "Reentrant cross-VM call");
        
        contexts[ctxHash] = ExecutionContext({
            gasLimit: gasleft(),
            computeUnitLimit: getRemainingComputeUnits(),
            callHash: ctxHash,
            timestamp: block.timestamp,
            caller: msg.sender
        });
        
        _;
        
        delete contexts[ctxHash];
    }
}
```

**Why this matters**: Cross-VM guards prevent reentrancy attacks that could exploit the bridge between VMs.

## Consensus Security

### Aura Consensus Security

X3 Chain uses Aura for block authoring with the following security properties:

**Validator Selection**: 
- Fixed set of validators during development
- Staking-based selection in production
- Economic incentives for honest behavior

**Finality**: 
- GRANDPA provides probabilistic finality
- 2 blocks (~12 seconds) for practical finality
- Byzantine fault tolerance up to 1/3 malicious validators

**Why this matters**: Proven consensus mechanism provides strong finality guarantees while enabling fast block times.

### Economic Security

**Validator Staking**:
- Minimum stake requirements
- Slashing for malicious behavior
- Reward distribution for honest participation

**Transaction Fees**:
- Prevents spam and DoS attacks
- Incentivizes validator participation
- Dynamic fee adjustment based on network load

**Why this matters**: Economic incentives align validator behavior with network security and performance.

## Runtime Security

### Memory Isolation

Each VM executes in isolated memory spaces:

```
┌─────────────────────────────────────────┐
│            EVM MEMORY                   │
│  ┌─────────────┐ ┌─────────────────────┐ │
│  │ Call Stack  │ │ Heap (dynamic)      │ │
│  └─────────────┘ └─────────────────────┘ │
│  ┌─────────────┐ ┌─────────────────────┐ │
│  │ Memory      │ │ Code (read-only)    │ │
│  │ (linear)    │ │                     │ │
│  └─────────────┘ └─────────────────────┘ │
└─────────────────────────────────────────┘
                     │
                     │ (no direct access)
                     ▼
┌─────────────────────────────────────────┐
│            SVM MEMORY                   │
│  ┌─────────────┐ ┌─────────────────────┐ │
│  │ Stack       │ │ Heap (account data) │ │
│  └─────────────┘ └─────────────────────┘ │
│  ┌─────────────┐ ┌─────────────────────┐ │
│  │ Call frames │ │ Program code (BPF)  │ │
│  └─────────────┘ └─────────────────────┘ │
└─────────────────────────────────────────┘
```

**Why this matters**: Memory isolation prevents VMs from accessing each other's memory directly, ensuring security boundaries.

### State Isolation

VMs maintain separate state trees synchronized via canonical ledger:

```
┌─────────────────────────────────────────────────────────┐
│                CANONICAL LEDGER                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  EVM State Tree                    SVM State Tree       │
│  ┌─────────────┐                    ┌──────────────┐    │
│  │ Accounts    │    Bridge          │ Accounts     │    │
│  │ Storage     │◄─────────────────►│ Programs     │    │
│  │ Code        │    Validation      │ Data         │    │
│  └─────────────┘                    └──────────────┘    │
│                                                         │
│  Cross-VM Validation:                                   │
│  • State root consistency checks                        │
│  • Asset balance verification                           │
│  • Event correlation validation                         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Why this matters**: State isolation with validation ensures that cross-VM operations maintain consistency.

## Developer Security Best Practices

### EVM Contract Security

**Follow Ethereum Security Patterns**:
```solidity
// Use checks-effects-interactions pattern
function transfer(address to, uint256 amount) external {
    // 1. Checks
    require(to != address(0), "Invalid address");
    require(balances[msg.sender] >= amount, "Insufficient balance");
    
    // 2. Effects
    balances[msg.sender] -= amount;
    balances[to] += amount;
    
    // 3. Interactions
    emit Transfer(msg.sender, to, amount);
}
```

**Avoid Common Vulnerabilities**:
- Reentrancy attacks
- Integer overflow/underflow
- Access control issues
- frontrunning

**Why this matters**: Standard Ethereum security patterns are proven and battle-tested.

### SVM Program Security

**Follow Solana Security Patterns**:
```rust
pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
    // Validate accounts
    require!(ctx.accounts.from.key() != ctx.accounts.to.key(), InvalidAccounts);
    
    // Check authority
    require!(
        ctx.accounts.from.owner == ctx.accounts.authority.key(),
        Unauthorized
    );
    
    // Perform transfer
    let from_balance = &mut ctx.accounts.from.balance;
    require!(*from_balance >= amount, InsufficientFunds);
    *from_balance -= amount;
    ctx.accounts.to.balance += amount;
    
    Ok(())
}
```

**Account Validation**:
- Verify account ownership
- Check account relationships
- Validate program-derived addresses

**Why this matters**: Solana security patterns account for the unique account-based model.

### Cross-VM Security Patterns

**Validate Cross-VM Calls**:
```solidity
function executeCrossVmOperation(
    address svmProgram,
    bytes memory svmData
) external {
    // 1. Validate SVM program address
    require(isAuthorizedSvmProgram[svmProgram], "Unauthorized program");
    
    // 2. Encode and validate call data
    bytes memory callData = abi.encode(svmProgram, "method", svmData);
    require(callData.length <= MAX_CALL_DATA_SIZE, "Data too large");
    
    // 3. Execute with gas limit
    uint256 gasBefore = gasleft();
    (bool success, bytes memory result) = crossVmCall(svmProgram, "method", svmData);
    uint256 gasUsed = gasBefore - gasleft();
    
    // 4. Validate result
    require(success, "Cross-VM call failed");
    require(gasUsed <= MAX_CROSS_VM_GAS, "Gas limit exceeded");
    
    // 5. Process result
    processResult(result);
}
```

**State Validation**:
```solidity
function validateCrossVmState(
    bytes32 evmStateRoot,
    bytes32 svmStateRoot,
    bytes32 expectedRoot
) internal pure {
    require(
        keccak256(abi.encode(evmStateRoot, svmStateRoot)) == expectedRoot,
        "State mismatch"
    );
}
```

**Why this matters**: Cross-VM security requires additional validation beyond single-VM patterns.

## Audit Considerations

### Pre-Audit Checklist

**Code Review**:
- [ ] All external function calls have access controls
- [ ] Reentrancy guards implemented where needed
- [ ] Integer overflow/underflow checks in place
- [ ] Cross-VM calls properly validated
- [ ] State consistency maintained across VMs

**Testing**:
- [ ] Unit tests cover all code paths
- [ ] Integration tests verify cross-VM behavior
- [ ] Property tests validate invariants
- [ ] Fuzz testing for edge cases
- [ ] Gas consumption tests

**Static Analysis**:
- [ ] Slither analysis (EVM contracts)
- [ ] Seahorse analysis (SVM programs)
- [ ] Custom cross-VM analysis rules
- [ ] Formal verification where applicable

**Why this matters**: Comprehensive audit preparation increases likelihood of finding security issues before deployment.

### Automated Security Tools

**EVM Tools**:
```bash
# Static analysis
slither contracts/

# Gas optimization
mythril analyze contracts/Contract.sol

# Formal verification
certora-prover contracts/Contract.sol
```

**SVM Tools**:
```bash
# Anchor checks
anchor check

# BPF verification
solana-verify verify PROGRAM_ID

# Custom security analysis
cargo audit
```

**Why this matters**: Automated tools catch common vulnerabilities that manual review might miss.

## Incident Response

### Security Incident Handling

**Detection**:
- Real-time monitoring of network anomalies
- Automated alerts for suspicious transactions
- Community reporting channels

**Response**:
1. **Immediate**: Pause affected functionality if needed
2. **Assessment**: Determine scope and impact
3. **Mitigation**: Implement fixes or workarounds
4. **Communication**: Notify users and stakeholders
5. **Recovery**: Restore normal operations
6. **Post-Mortem**: Document lessons learned

**Why this matters**: Prepared incident response minimizes damage and builds trust.

### Emergency Procedures

**Network Upgrades**:
- Runtime upgrade mechanism for security patches
- Emergency halt capability
- Validator coordination protocols

**User Communication**:
- Security advisories
- Status page updates
- Social media notifications

**Why this matters**: Clear communication during incidents prevents panic and misinformation.

## Security Contact

For security-related issues:

**Email**: security@x3-chain.io
**PGP Key**: Available at security.x3-chain.io/pgp
**Bug Bounty**: [bounty.x3-chain.io](https://bounty.x3-chain.io)
**Response Time**: 24 hours for critical issues

**Why this matters**: Multiple contact methods ensure security researchers can reach the team through preferred channels.

---

*This security model is continuously evolving. For the latest information, see our [Security Updates](https://security.x3-chain.io) page.*
