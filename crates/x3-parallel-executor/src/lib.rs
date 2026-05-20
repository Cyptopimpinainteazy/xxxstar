//! X3 Parallel Executor
//!
//! Deterministic parallel execution with serial equivalence proofs.
//! Executes transactions in parallel while maintaining the same final state
//! as serial execution, proven through conflict detection and ordering.

pub mod types;

use sp_io::hashing::blake2_256;
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
use sp_std::vec::Vec;

/// Parallel execution scheduler with conflict detection
pub struct ParallelExecutor {
    /// Maximum concurrent transactions
    #[allow(dead_code)]
    max_concurrent: usize,
    /// Conflict detector
    conflict_detector: ConflictDetector,
    /// Access list builder
    access_list: AccessListBuilder,
}

impl ParallelExecutor {
    /// Create new parallel executor
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            conflict_detector: ConflictDetector::new(),
            access_list: AccessListBuilder::new(),
        }
    }

    /// Execute transactions in parallel with serial equivalence
    pub fn execute_parallel(
        &self,
        transactions: Vec<Transaction>,
    ) -> Result<ExecutionResult, ExecutionError> {
        // Phase 1: Build access lists for each transaction
        let access_lists: Vec<AccessList> = transactions
            .iter()
            .map(|tx| self.access_list.build_access_list(tx))
            .collect();

        // Phase 2: Detect conflicts between transactions
        let conflicts = self.conflict_detector.detect_conflicts(&access_lists)?;

        // Phase 3: Build dependency graph and execution order
        let execution_order = self.build_execution_order(&transactions, &conflicts)?;

        // Phase 4: Execute in parallel respecting dependencies
        let results = self.execute_with_dependencies(execution_order, &transactions)?;

        // Phase 5: Verify serial equivalence
        self.verify_serial_equivalence(&transactions, &results)?;

        Ok(results)
    }

    /// Build execution order respecting dependencies
    fn build_execution_order(
        &self,
        transactions: &[Transaction],
        conflicts: &[Conflict],
    ) -> Result<Vec<ExecutionBatch>, ExecutionError> {
        let mut batches = Vec::new();
        let mut processed = BTreeSet::new();

        for i in 0..transactions.len() {
            if processed.contains(&i) {
                continue;
            }

            let mut batch = Vec::new();
            batch.push(i);

            // Add non-conflicting transactions to same batch
            for j in 0..transactions.len() {
                if i == j || processed.contains(&j) {
                    continue;
                }

                if !self.conflicts_with_batch(j, &batch, conflicts) {
                    batch.push(j);
                    processed.insert(j);
                }
            }

            batches.push(ExecutionBatch {
                transaction_indices: batch,
            });
            processed.insert(i);
        }

        Ok(batches)
    }

    /// Check if transaction conflicts with any in batch
    fn conflicts_with_batch(
        &self,
        tx_index: usize,
        batch: &[usize],
        conflicts: &[Conflict],
    ) -> bool {
        batch.iter().any(|batch_index| {
            conflicts
                .iter()
                .any(|conflict| conflict.involves(tx_index, *batch_index))
        })
    }

    /// Execute batches respecting dependencies
    fn execute_with_dependencies(
        &self,
        batches: Vec<ExecutionBatch>,
        transactions: &[Transaction],
    ) -> Result<ExecutionResult, ExecutionError> {
        let mut results = ExecutionResult::new();

        for batch in batches {
            // Execute batch transactions in parallel
            let batch_results = self.execute_batch(batch, transactions)?;

            // Merge results maintaining order
            for result in batch_results {
                results.merge(result)?;
            }

            // Commit batch state changes
            results.commit_batch()?;
        }

        Ok(results)
    }

    /// Execute a single batch of transactions
    fn execute_batch(
        &self,
        batch: ExecutionBatch,
        transactions: &[Transaction],
    ) -> Result<Vec<TransactionResult>, ExecutionError> {
        // In parallel execution, we'd spawn tasks here
        // For now, simulate parallel execution
        batch
            .transaction_indices
            .into_iter()
            .map(|index| self.execute_transaction(transactions[index].clone()))
            .collect()
    }

    /// Execute single transaction
    fn execute_transaction(&self, tx: Transaction) -> Result<TransactionResult, ExecutionError> {
        let mut state_changes = Vec::new();
        let mut events = Vec::new();

        for instruction in tx.instructions.iter() {
            match instruction.opcode {
                0x01 => {
                    events.push(Event {
                        topic: b"read".to_vec(),
                        data: instruction.operands.clone(),
                    });
                }
                0x02 | 0x03 => {
                    let key = Self::state_key_for_instruction(instruction);
                    state_changes.push(StateChange {
                        key,
                        old_value: Vec::new(),
                        new_value: instruction.operands.clone(),
                    });
                }
                _ => {
                    events.push(Event {
                        topic: b"noop".to_vec(),
                        data: instruction.operands.clone(),
                    });
                }
            }
        }

        Ok(TransactionResult {
            tx_id: tx.id,
            success: true,
            state_changes,
            events,
        })
    }

    fn state_key_for_instruction(instruction: &Instruction) -> [u8; 32] {
        let mut buffer = Vec::with_capacity(1 + instruction.operands.len());
        buffer.push(instruction.opcode);
        buffer.extend_from_slice(&instruction.operands);
        blake2_256(&buffer)
    }

    /// Verify parallel results match serial execution
    fn verify_serial_equivalence(
        &self,
        transactions: &[Transaction],
        parallel_result: &ExecutionResult,
    ) -> Result<(), ExecutionError> {
        // Simulate serial execution
        let mut serial_result = ExecutionResult::new();
        for tx in transactions {
            let result = self.execute_transaction(tx.clone())?;
            serial_result.merge(result)?;
            serial_result.commit_batch()?;
        }

        // Compare final states
        if serial_result.final_state_hash() == parallel_result.final_state_hash() {
            Ok(())
        } else {
            Err(ExecutionError::SerialEquivalenceViolation)
        }
    }
}

/// Conflict detection for parallel execution
pub struct ConflictDetector {
    /// Read-write conflict tracking
    #[allow(dead_code)]
    rw_conflicts: BTreeMap<StateKey, Vec<TransactionId>>,
}

impl ConflictDetector {
    pub fn new() -> Self {
        Self {
            rw_conflicts: BTreeMap::new(),
        }
    }

    /// Detect conflicts between access lists
    pub fn detect_conflicts(
        &self,
        access_lists: &[AccessList],
    ) -> Result<Vec<Conflict>, ExecutionError> {
        let mut conflicts = Vec::new();

        for i in 0..access_lists.len() {
            for j in (i + 1)..access_lists.len() {
                if let Some(conflict) =
                    self.check_conflict(&access_lists[i], &access_lists[j], i, j)
                {
                    conflicts.push(conflict);
                }
            }
        }

        Ok(conflicts)
    }

    /// Check if two access lists conflict
    fn check_conflict(
        &self,
        list1: &AccessList,
        list2: &AccessList,
        tx1_idx: usize,
        tx2_idx: usize,
    ) -> Option<Conflict> {
        if list1.conflicts_with(list2) {
            // Pick a representative conflicting key for diagnostics.
            for write in &list1.writes {
                if list2.reads.contains(write) || list2.writes.contains(write) {
                    return Some(Conflict::new(tx1_idx, tx2_idx, *write));
                }
            }

            for write in &list2.writes {
                if list1.reads.contains(write) {
                    return Some(Conflict::new(tx1_idx, tx2_idx, *write));
                }
            }
        }

        None
    }
}

/// Access list builder for transactions
pub struct AccessListBuilder;

impl AccessListBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build access list for transaction
    pub fn build_access_list(&self, tx: &Transaction) -> AccessList {
        let mut reads = Vec::new();
        let mut writes = Vec::new();

        for instruction in tx.instructions.iter() {
            let key = ParallelExecutor::state_key_for_instruction(instruction);
            match instruction.opcode {
                0x01 => reads.push(key),
                0x02 => writes.push(key),
                0x03 => {
                    reads.push(key);
                    writes.push(key);
                }
                _ => {}
            }
        }

        AccessList { reads, writes }
    }
}

// Type definitions
pub type TransactionId = u64;
pub type StateKey = [u8; 32];

#[derive(Clone, Debug)]
pub struct Transaction {
    pub id: TransactionId,
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Debug)]
pub struct AccessList {
    pub reads: Vec<StateKey>,
    pub writes: Vec<StateKey>,
}

impl AccessList {
    pub fn conflicts_with(&self, other: &AccessList) -> bool {
        let my_reads: BTreeSet<_> = self.reads.iter().collect();
        let my_writes: BTreeSet<_> = self.writes.iter().collect();
        let other_reads: BTreeSet<_> = other.reads.iter().collect();
        let other_writes: BTreeSet<_> = other.writes.iter().collect();

        for write in other_writes.iter() {
            if my_reads.contains(write) || my_writes.contains(write) {
                return true;
            }
        }

        for write in my_writes.iter() {
            if other_reads.contains(write) {
                return true;
            }
        }

        false
    }
}

#[derive(Clone, Debug)]
pub struct Conflict {
    pub tx1: usize,
    pub tx2: usize,
    pub key: StateKey,
}

impl Conflict {
    pub fn new(tx1: usize, tx2: usize, key: StateKey) -> Self {
        Self { tx1, tx2, key }
    }

    pub fn involves(&self, tx_index1: usize, tx_index2: usize) -> bool {
        self.tx1 == tx_index1 && self.tx2 == tx_index2
            || self.tx1 == tx_index2 && self.tx2 == tx_index1
    }
}

#[derive(Clone, Debug)]
pub struct ExecutionBatch {
    pub transaction_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct TransactionResult {
    pub tx_id: TransactionId,
    pub success: bool,
    pub state_changes: Vec<StateChange>,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub results: Vec<TransactionResult>,
    pub state_hash: [u8; 32],
}

impl ExecutionResult {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            state_hash: [0; 32],
        }
    }

    pub fn merge(&mut self, result: TransactionResult) -> Result<(), ExecutionError> {
        self.results.push(result);
        self.recalculate_state_hash();
        Ok(())
    }

    pub fn commit_batch(&mut self) -> Result<(), ExecutionError> {
        Ok(())
    }

    fn recalculate_state_hash(&mut self) {
        let mut entropy = Vec::new();

        for result in self.results.iter() {
            entropy.extend_from_slice(&result.tx_id.to_le_bytes());
            entropy.push(if result.success { 1 } else { 0 });
            for state_change in result.state_changes.iter() {
                entropy.extend_from_slice(&state_change.key);
                entropy.extend_from_slice(&(state_change.new_value.len() as u32).to_le_bytes());
                entropy.extend_from_slice(&state_change.new_value);
            }
        }

        self.state_hash = blake2_256(&entropy);
    }

    pub fn final_state_hash(&self) -> [u8; 32] {
        self.state_hash
    }
}

#[derive(Clone, Debug)]
pub struct StateChange {
    pub key: StateKey,
    pub old_value: Vec<u8>,
    pub new_value: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub topic: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Instruction {
    pub opcode: u8,
    pub operands: Vec<u8>,
}

#[derive(Debug)]
pub enum ExecutionError {
    ConflictDetected,
    SerialEquivalenceViolation,
    TransactionFailed,
    StateCorruption,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_executor_creation() {
        let executor = ParallelExecutor::new(4);
        assert_eq!(executor.max_concurrent, 4);
    }

    #[test]
    fn test_conflict_detection() {
        let detector = ConflictDetector::new();

        let list1 = AccessList {
            reads: vec![],
            writes: vec![[1; 32]],
        };

        let list2 = AccessList {
            reads: vec![[1; 32]],
            writes: vec![],
        };

        let conflicts = detector.detect_conflicts(&[list1, list2]).unwrap();
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_serial_equivalence() {
        let executor = ParallelExecutor::new(4);
        let transactions = vec![Transaction {
            id: 1,
            instructions: vec![],
        }];

        let result = executor.execute_parallel(transactions);
        assert!(result.is_ok());
    }

    #[test]
    fn test_access_list_builder() {
        let builder = AccessListBuilder::new();
        let tx = Transaction {
            id: 1,
            instructions: vec![
                Instruction {
                    opcode: 1,
                    operands: vec![1, 2, 3],
                },
                Instruction {
                    opcode: 2,
                    operands: vec![4, 5, 6],
                },
            ],
        };

        let access_list = builder.build_access_list(&tx);
        // Access list should be empty for now (simplified implementation)
        assert_eq!(access_list.reads.len(), 0);
        assert_eq!(access_list.writes.len(), 0);
    }

    #[test]
    fn test_conflict_new() {
        let conflict = Conflict::new(0, 1, [42; 32]);
        assert_eq!(conflict.tx1, 0);
        assert_eq!(conflict.tx2, 1);
        assert_eq!(conflict.key, [42; 32]);
    }

    #[test]
    fn test_execution_batch_creation() {
        let batch = ExecutionBatch {
            transaction_indices: vec![0, 1],
        };
        assert_eq!(batch.transaction_indices.len(), 2);
        assert_eq!(batch.transaction_indices[0], 0);
        assert_eq!(batch.transaction_indices[1], 1);
    }

    #[test]
    fn test_transaction_result_creation() {
        let state_changes = vec![StateChange {
            key: [1; 32],
            old_value: vec![0],
            new_value: vec![1],
        }];

        let events = vec![Event {
            topic: vec![1, 2, 3],
            data: vec![4, 5, 6],
        }];

        let result = TransactionResult {
            tx_id: 42,
            success: true,
            state_changes,
            events,
        };

        assert_eq!(result.tx_id, 42);
        assert!(result.success);
        assert_eq!(result.state_changes.len(), 1);
        assert_eq!(result.events.len(), 1);
    }

    #[test]
    fn test_execution_result_operations() {
        let mut result = ExecutionResult::new();
        assert_eq!(result.results.len(), 0);

        let tx_result = TransactionResult {
            tx_id: 1,
            success: true,
            state_changes: vec![],
            events: vec![],
        };

        result.merge(tx_result).unwrap();
        assert_eq!(result.results.len(), 1);

        result.commit_batch().unwrap();
        assert_eq!(result.final_state_hash(), [0; 32]);
    }

    #[test]
    fn test_instruction_creation() {
        let instruction = Instruction {
            opcode: 0xAB,
            operands: vec![1, 2, 3, 4],
        };

        assert_eq!(instruction.opcode, 0xAB);
        assert_eq!(instruction.operands, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_state_change_creation() {
        let change = StateChange {
            key: [255; 32],
            old_value: vec![0, 1, 2],
            new_value: vec![3, 4, 5],
        };

        assert_eq!(change.key, [255; 32]);
        assert_eq!(change.old_value, vec![0, 1, 2]);
        assert_eq!(change.new_value, vec![3, 4, 5]);
    }

    #[test]
    fn test_event_creation() {
        let event = Event {
            topic: b"transfer".to_vec(),
            data: vec![1, 2, 3, 4, 5],
        };

        assert_eq!(event.topic, b"transfer");
        assert_eq!(event.data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_no_conflicts_when_independent() {
        let detector = ConflictDetector::new();

        let list1 = AccessList {
            reads: vec![[1; 32]],
            writes: vec![[2; 32]],
        };

        let list2 = AccessList {
            reads: vec![[3; 32]],
            writes: vec![[4; 32]],
        };

        let conflicts = detector.detect_conflicts(&[list1, list2]).unwrap();
        assert_eq!(conflicts.len(), 0);
    }

    #[test]
    fn test_write_write_conflict() {
        let detector = ConflictDetector::new();

        let list1 = AccessList {
            reads: vec![],
            writes: vec![[1; 32]],
        };

        let list2 = AccessList {
            reads: vec![],
            writes: vec![[1; 32]], // Same key
        };

        let conflicts = detector.detect_conflicts(&[list1, list2]).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].key, [1; 32]);
    }

    #[test]
    fn test_read_write_conflict() {
        let detector = ConflictDetector::new();

        let list1 = AccessList {
            reads: vec![[1; 32]],
            writes: vec![],
        };

        let list2 = AccessList {
            reads: vec![],
            writes: vec![[1; 32]], // Write conflicts with read
        };

        let conflicts = detector.detect_conflicts(&[list1, list2]).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].key, [1; 32]);
    }

    #[test]
    fn test_multiple_conflicts() {
        let detector = ConflictDetector::new();

        let list1 = AccessList {
            reads: vec![],
            writes: vec![[1; 32], [2; 32]],
        };

        let list2 = AccessList {
            reads: vec![[1; 32], [2; 32]],
            writes: vec![[3; 32]],
        };

        let list3 = AccessList {
            reads: vec![[3; 32]],
            writes: vec![[4; 32]],
        };

        let conflicts = detector.detect_conflicts(&[list1, list2, list3]).unwrap();
        assert_eq!(conflicts.len(), 3); // Conflicts between 1-2, 2-3, and 1-2 for key 2
    }

    #[test]
    fn test_empty_access_lists() {
        let detector = ConflictDetector::new();
        let conflicts = detector.detect_conflicts(&[]).unwrap();
        assert_eq!(conflicts.len(), 0);
    }

    #[test]
    fn test_parallel_execution_with_no_conflicts() {
        let executor = ParallelExecutor::new(4);

        let transactions = vec![
            Transaction {
                id: 1,
                instructions: vec![Instruction {
                    opcode: 1,
                    operands: vec![1],
                }],
            },
            Transaction {
                id: 2,
                instructions: vec![Instruction {
                    opcode: 2,
                    operands: vec![2],
                }],
            },
        ];

        let result = executor.execute_parallel(transactions);
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.results.len(), 2);
    }

    #[test]
    fn test_parallel_execution_with_conflicts() {
        let executor = ParallelExecutor::new(4);

        // Create transactions that would conflict
        let tx1 = Transaction {
            id: 1,
            instructions: vec![Instruction {
                opcode: 1,
                operands: vec![1],
            }],
        };

        let tx2 = Transaction {
            id: 2,
            instructions: vec![Instruction {
                opcode: 1,
                operands: vec![1],
            }], // Same operation
        };

        let transactions = vec![tx1, tx2];
        let result = executor.execute_parallel(transactions);
        // Should still succeed as conflict detection handles it
        assert!(result.is_ok());
    }

    #[test]
    fn test_execution_batch_empty() {
        let batch = ExecutionBatch {
            transaction_indices: vec![],
        };
        assert_eq!(batch.transaction_indices.len(), 0);
    }

    #[test]
    fn test_transaction_empty_instructions() {
        let tx = Transaction {
            id: 42,
            instructions: vec![],
        };

        assert_eq!(tx.id, 42);
        assert_eq!(tx.instructions.len(), 0);
    }

    #[test]
    fn test_access_list_empty() {
        let access_list = AccessList {
            reads: vec![],
            writes: vec![],
        };

        assert_eq!(access_list.reads.len(), 0);
        assert_eq!(access_list.writes.len(), 0);
    }
}
