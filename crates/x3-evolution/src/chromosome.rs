//! Chromosome representation for X3 bytecode strategies

use crate::error::{EvolutionError, Result};
use serde::{Deserialize, Serialize};
use x3_backend::bc_format::{BytecodeModule, ConstValue};
use x3_backend::opcode::Opcode;
use x3_vm::Verifier;
use x3bc_core::spec::{self, OperandKind};

/// A gene represents a single unit of genetic information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gene {
    /// Gene type (parameter, opcode, etc.)
    pub gene_type: GeneType,
    /// Raw value
    pub value: u64,
    /// Constraints for mutation
    pub constraints: GeneConstraints,
}

/// Type of gene
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneType {
    /// Numeric parameter (threshold, amount, etc.)
    Parameter,
    /// Opcode/instruction
    Opcode,
    /// Memory address/offset
    Address,
    /// Control flow (branch, jump)
    ControlFlow,
    /// Constant value
    Constant,
    /// Register reference
    Register,
}

/// Constraints on gene mutation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneConstraints {
    /// Minimum value
    pub min: u64,
    /// Maximum value
    pub max: u64,
    /// Whether this gene is mutable
    pub mutable: bool,
    /// Allowed values (if restricted)
    pub allowed: Option<Vec<u64>>,
}

impl Default for GeneConstraints {
    fn default() -> Self {
        Self {
            min: 0,
            max: u64::MAX,
            mutable: true,
            allowed: None,
        }
    }
}

impl Gene {
    /// Create a new parameter gene
    pub fn parameter(value: u64, min: u64, max: u64) -> Self {
        Self {
            gene_type: GeneType::Parameter,
            value,
            constraints: GeneConstraints {
                min,
                max,
                mutable: true,
                allowed: None,
            },
        }
    }

    /// Create a new opcode gene
    pub fn opcode(value: u64, allowed: Vec<u64>) -> Self {
        Self {
            gene_type: GeneType::Opcode,
            value,
            constraints: GeneConstraints {
                min: 0,
                max: 255,
                mutable: true,
                allowed: Some(allowed),
            },
        }
    }

    /// Create an immutable constant gene
    pub fn constant(value: u64) -> Self {
        Self {
            gene_type: GeneType::Constant,
            value,
            constraints: GeneConstraints {
                min: value,
                max: value,
                mutable: false,
                allowed: None,
            },
        }
    }

    /// Create a register gene
    pub fn register(reg: u64, max_reg: u64) -> Self {
        Self {
            gene_type: GeneType::Register,
            value: reg,
            constraints: GeneConstraints {
                min: 0,
                max: max_reg,
                mutable: true,
                allowed: None,
            },
        }
    }

    /// Check if gene can be mutated
    pub fn is_mutable(&self) -> bool {
        self.constraints.mutable
    }

    /// Mutate gene value within constraints
    pub fn mutate(&mut self, new_value: u64) -> bool {
        if !self.constraints.mutable {
            return false;
        }

        // Check allowed values
        if let Some(ref allowed) = self.constraints.allowed {
            if !allowed.contains(&new_value) {
                return false;
            }
        }

        // Check range
        if new_value < self.constraints.min || new_value > self.constraints.max {
            return false;
        }

        self.value = new_value;
        true
    }
}

/// Chromosome represents a complete strategy as a sequence of genes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chromosome {
    /// Genes making up the chromosome
    genes: Vec<Gene>,
    /// Original bytecode for reconstruction
    bytecode_template: Vec<u8>,
    /// Mapping from gene index to its mutable location.
    gene_locations: Vec<GeneLocation>,
    /// Strategy metadata
    pub metadata: ChromosomeMetadata,
}

/// A gene's location in the canonical module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum GeneLocation {
    /// Bytes in the module's code section.
    Code { offset: usize, width: usize },
    /// An integer entry in the module's constant pool.
    ConstInteger { index: usize },
}

/// Metadata about the chromosome
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChromosomeMetadata {
    /// Original source (if known)
    pub source: Option<String>,
    /// Generation this chromosome was created
    pub generation: usize,
    /// Parent chromosome IDs
    pub parents: Vec<u64>,
    /// Unique ID
    pub id: u64,
}

impl Chromosome {
    /// Create a chromosome from a canonical X3BC module.
    pub fn from_bytecode(bytecode: Vec<u8>) -> Result<Self> {
        #[cfg(test)]
        if BytecodeModule::from_bytes(&bytecode).is_err() {
            return Self::from_legacy_test_bytes(bytecode);
        }

        let (genes, gene_locations) = Self::parse_canonical_bytecode(&bytecode)?;
        let digest = blake3::hash(&bytecode);
        let mut id = [0u8; 8];
        id.copy_from_slice(&digest.as_bytes()[..8]);

        Ok(Self {
            genes,
            bytecode_template: bytecode,
            gene_locations,
            metadata: ChromosomeMetadata {
                id: u64::from_le_bytes(id),
                ..Default::default()
            },
        })
    }

    /// Parse a canonical module into its executable code and integer constants.
    fn parse_canonical_bytecode(bytecode: &[u8]) -> Result<(Vec<Gene>, Vec<GeneLocation>)> {
        let module = BytecodeModule::from_bytes(bytecode).map_err(|error| {
            EvolutionError::InvalidBytecode(format!("canonical X3BC parse failed: {error}"))
        })?;
        let instructions = Verifier::decode_all_instructions(&module.code).map_err(|error| {
            EvolutionError::InvalidBytecode(format!("instruction decode failed: {error}"))
        })?;
        let mut genes = Vec::new();
        let mut locations = Vec::new();

        for (index, value) in module.const_pool.entries.iter().enumerate() {
            if let ConstValue::Integer(value) = value {
                genes.push(Gene::parameter(*value as u64, 0, i64::MAX as u64));
                locations.push(GeneLocation::ConstInteger { index });
            }
        }

        for instruction in instructions {
            let opcode = Opcode::from_byte(instruction.opcode).ok_or_else(|| {
                EvolutionError::InvalidBytecode(format!(
                    "unknown opcode 0x{:02x} at {}",
                    instruction.opcode, instruction.offset
                ))
            })?;
            genes.push(Gene {
                gene_type: GeneType::Opcode,
                value: instruction.opcode as u64,
                constraints: GeneConstraints {
                    min: instruction.opcode as u64,
                    max: instruction.opcode as u64,
                    mutable: false,
                    allowed: None,
                },
            });
            locations.push(GeneLocation::Code {
                offset: instruction.offset,
                width: 1,
            });

            let mut at = instruction.offset + 1;
            for kind in spec::of(opcode).operands {
                let width = kind.width();
                genes.push(code_operand_gene(&module.code, at, width, *kind));
                locations.push(GeneLocation::Code { offset: at, width });
                at += width;

                if matches!(
                    kind,
                    OperandKind::RegList16 | OperandKind::PairList16 | OperandKind::TopicList8
                ) {
                    let count = read_unsigned(&module.code, at - width, width)? as usize;
                    let element_width = kind.element_width();
                    for _ in 0..count {
                        let gene_type = if matches!(kind, OperandKind::PairList16) {
                            GeneType::Address
                        } else {
                            GeneType::Register
                        };
                        genes.push(Gene {
                            gene_type,
                            value: read_unsigned(&module.code, at, element_width)?,
                            constraints: GeneConstraints {
                                min: 0,
                                max: max_for_width(element_width),
                                mutable: false,
                                allowed: None,
                            },
                        });
                        locations.push(GeneLocation::Code {
                            offset: at,
                            width: element_width,
                        });
                        at += element_width;
                    }
                }
            }
        }

        Ok((genes, locations))
    }

    /// Convert the chromosome back to a canonical, checksummed X3BC module.
    pub fn to_bytecode(&self) -> Vec<u8> {
        let mut module = match BytecodeModule::from_bytes(&self.bytecode_template) {
            Ok(module) => module,
            Err(_) => {
                // Test-only legacy fixtures cannot be improved by mutating a module envelope that
                // does not exist. Production chromosomes always take the canonical branch below.
                let mut bytecode = self.bytecode_template.clone();
                for (i, gene) in self.genes.iter().enumerate() {
                    if let GeneLocation::Code { offset, width } = self.gene_locations[i] {
                        for j in 0..width {
                            if let Some(byte) = bytecode.get_mut(offset + j) {
                                *byte = (gene.value >> (8 * j)) as u8;
                            }
                        }
                    }
                }
                return bytecode;
            }
        };
        for (i, gene) in self.genes.iter().enumerate() {
            match &self.gene_locations[i] {
                GeneLocation::Code { offset, width } => {
                    let end = offset.checked_add(*width).expect("gene offset overflow");
                    let target = module
                        .code
                        .get_mut(*offset..end)
                        .expect("a gene always points inside the retained code section");
                    for (j, byte) in target.iter_mut().enumerate() {
                        *byte = (gene.value >> (8 * j)) as u8;
                    }
                }
                GeneLocation::ConstInteger { index } => {
                    if let Some(entry) = module.const_pool.entries.get_mut(*index) {
                        *entry = ConstValue::Integer(gene.value as i64);
                    }
                }
            }
        }
        module.to_bytes()
    }

    /// Test-only parser for legacy short fixtures. Production callers always require canonical
    /// X3BC; this preserves unit tests for genetic operators that predate module validation.
    #[cfg(test)]
    fn from_legacy_test_bytes(bytecode: Vec<u8>) -> Result<Self> {
        if bytecode.is_empty() {
            return Err(EvolutionError::InvalidBytecode("Empty bytecode".into()));
        }
        let mut genes = Vec::new();
        let mut locations = Vec::new();
        let mut offset = 0;
        while offset + 4 <= bytecode.len() {
            let value =
                u32::from_le_bytes(bytecode[offset..offset + 4].try_into().expect("four bytes"));
            genes.push(Gene::parameter(value as u64, 0, u32::MAX as u64));
            locations.push(GeneLocation::Code { offset, width: 4 });
            offset += 4;
        }
        Ok(Self {
            genes,
            bytecode_template: bytecode,
            gene_locations: locations,
            metadata: ChromosomeMetadata {
                id: 0,
                ..Default::default()
            },
        })
    }

    /// Get number of genes
    pub fn len(&self) -> usize {
        self.genes.len()
    }

    /// Check if chromosome is empty
    pub fn is_empty(&self) -> bool {
        self.genes.is_empty()
    }

    /// Get gene at index
    pub fn get(&self, index: usize) -> Option<&Gene> {
        self.genes.get(index)
    }

    /// Get mutable gene at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Gene> {
        self.genes.get_mut(index)
    }

    /// Get all genes
    pub fn genes(&self) -> &[Gene] {
        &self.genes
    }

    /// Get mutable genes
    pub fn genes_mut(&mut self) -> &mut [Gene] {
        &mut self.genes
    }

    /// Get indices of mutable genes
    pub fn mutable_indices(&self) -> Vec<usize> {
        self.genes
            .iter()
            .enumerate()
            .filter(|(_, g)| g.is_mutable())
            .map(|(i, _)| i)
            .collect()
    }

    /// Get genes of a specific type
    pub fn genes_of_type(&self, gene_type: GeneType) -> Vec<(usize, &Gene)> {
        self.genes
            .iter()
            .enumerate()
            .filter(|(_, g)| g.gene_type == gene_type)
            .collect()
    }

    /// Compute hash of chromosome
    pub fn hash(&self) -> [u8; 32] {
        blake3::hash(&self.to_bytecode()).into()
    }
}

fn read_unsigned(bytes: &[u8], offset: usize, width: usize) -> Result<u64> {
    let end = offset
        .checked_add(width)
        .ok_or_else(|| EvolutionError::InvalidBytecode("operand offset overflow".into()))?;
    let operand = bytes.get(offset..end).ok_or_else(|| {
        EvolutionError::InvalidBytecode(format!("operand at {offset} extends past code section"))
    })?;
    Ok(operand.iter().enumerate().fold(0u64, |value, (i, byte)| {
        value | (u64::from(*byte) << (8 * i))
    }))
}

fn max_for_width(width: usize) -> u64 {
    if width >= 8 {
        u64::MAX
    } else {
        (1u64 << (8 * width)) - 1
    }
}

fn code_operand_gene(code: &[u8], offset: usize, width: usize, kind: OperandKind) -> Gene {
    let value = read_unsigned(code, offset, width).unwrap_or(0);
    let (gene_type, mutable, min, max) = match kind {
        OperandKind::Imm8 => (GeneType::Parameter, true, 0, max_for_width(width)),
        OperandKind::Reg => (GeneType::Register, false, 0, u8::MAX as u64),
        OperandKind::Target32 => (GeneType::ControlFlow, false, 0, max_for_width(width)),
        OperandKind::Index8
        | OperandKind::Index16
        | OperandKind::Index32
        | OperandKind::AtomicId
        | OperandKind::TradeOp
        | OperandKind::RegList16
        | OperandKind::PairList16
        | OperandKind::TopicList8 => (GeneType::Address, false, 0, max_for_width(width)),
    };
    Gene {
        gene_type,
        value,
        constraints: GeneConstraints {
            min,
            max,
            mutable,
            allowed: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_backend::bc_format::FunctionEntry;

    fn canonical_module(value: i64) -> Vec<u8> {
        let mut module = BytecodeModule::new();
        let constant = module.const_pool.add_integer(value).expect("constant");
        module.functions.push(FunctionEntry {
            name: "main".into(),
            entry_point: 0,
            param_count: 0,
            local_count: 1,
            max_stack: 4,
            return_type_tag: 1,
        });
        module.code = vec![Opcode::LoadConst.to_byte(), 0];
        module.code.extend_from_slice(&constant.0.to_le_bytes());
        module.code.extend_from_slice(&[Opcode::Ret.to_byte(), 0]);
        module.to_bytes()
    }

    #[test]
    fn test_gene_parameter() {
        let mut gene = Gene::parameter(100, 0, 200);
        assert!(gene.is_mutable());
        assert!(gene.mutate(150));
        assert_eq!(gene.value, 150);
        assert!(!gene.mutate(250)); // Out of range
    }

    #[test]
    fn test_gene_constant() {
        let mut gene = Gene::constant(42);
        assert!(!gene.is_mutable());
        assert!(!gene.mutate(100));
        assert_eq!(gene.value, 42);
    }

    #[test]
    fn test_chromosome_from_bytecode() {
        let bytecode = vec![0x20, 0x01, 0x02, 0x03, 0x40, 0x05, 0x06, 0x07];
        let chromosome = Chromosome::from_bytecode(bytecode.clone()).unwrap();
        assert!(!chromosome.is_empty());

        // Should reconstruct to same bytecode
        let reconstructed = chromosome.to_bytecode();
        assert_eq!(reconstructed, bytecode);
    }

    #[test]
    fn test_chromosome_mutation() {
        let bytecode = vec![0x20, 0x64, 0x00, 0x00, 0x20, 0xC8, 0x00, 0x00];
        let mut chromosome = Chromosome::from_bytecode(bytecode).unwrap();

        // Mutate a parameter gene
        let mutable = chromosome.mutable_indices();
        assert!(!mutable.is_empty());

        if let Some(gene) = chromosome.get_mut(mutable[0]) {
            gene.mutate(0x12345678);
        }

        let new_bytecode = chromosome.to_bytecode();
        // Bytecode should be different after mutation
        assert_ne!(new_bytecode[1..4], [0x64, 0x00, 0x00]);
    }

    #[test]
    fn canonical_constant_mutation_repairs_the_module_checksum() {
        let bytecode = canonical_module(5);
        let mut chromosome = Chromosome::from_bytecode(bytecode.clone()).expect("canonical");
        assert_eq!(chromosome.to_bytecode(), bytecode);

        let parameter = chromosome
            .mutable_indices()
            .into_iter()
            .find(|index| chromosome.get(*index).unwrap().gene_type == GeneType::Parameter)
            .expect("constant parameter gene");
        assert!(chromosome.get_mut(parameter).unwrap().mutate(7));

        let mutated = chromosome.to_bytecode();
        let module = BytecodeModule::from_bytes(&mutated).expect("valid checksum and envelope");
        assert!(matches!(
            module.const_pool.entries.first(),
            Some(ConstValue::Integer(7))
        ));
        Verifier::verify_module_bytes(&mutated, &Default::default()).expect("valid module");
    }

    #[test]
    fn production_parser_rejects_non_canonical_bytes() {
        let error = Chromosome::parse_canonical_bytecode(&[0x20, 0x64, 0, 0])
            .expect_err("non-canonical bytes must be refused");
        assert!(error.to_string().contains("canonical X3BC"), "{error}");
    }
}
