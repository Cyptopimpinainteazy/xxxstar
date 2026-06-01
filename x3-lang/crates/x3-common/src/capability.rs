//! Typed binary payloads for X3 capability opcodes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityPayload {
    GpuDispatch {
        kernel: String,
        args: Vec<String>,
        is_simd: bool,
    },
    Simulate {
        body_ops: u32,
        receipt_slot: String,
    },
    ScheduledDispatch {
        period_blocks: u32,
        entry_ops: u32,
    },
    IntentResolve {
        constraints: Vec<String>,
        resolver: String,
    },
    CrdtOp {
        kind: u8,
        key: String,
        value: Option<String>,
    },
    ProofVerify {
        kind: u8,
        proof: String,
        input: String,
        key_or_threshold: String,
    },
    StorageOp {
        kind: u8,
        data: String,
    },
    Pathfind {
        from: String,
        to: String,
        max_depth: u32,
    },
    MempoolScan {
        max_results: u32,
    },
    OracleRequest {
        token: String,
        reward: u128,
    },
    EmergencyControl {
        kind: u8,
    },
    Lifecycle {
        kind: u8,
        target: Option<String>,
    },
    Serialize {
        format: u8,
        data: String,
    },
    Deserialize {
        format: u8,
        data: String,
    },
    GasEstimate {
        chain: String,
        route: String,
    },
    ChainMetric {
        metric: u8,
    },
    EventProvenance {
        event_type: String,
        data: String,
    },
    MultiHopSwap {
        path: Vec<String>,
        amount: u128,
    },
    VectorMath {
        op: u8,
        a: String,
        b: String,
        size: u32,
    },
    RoleCheck {
        role: String,
    },
    MultisigCheck {
        required: u32,
        total: u32,
    },
    VersionMeta {
        version: String,
        upgrade_from: Option<String>,
    },
    StorageNamespace {
        package: String,
        key: String,
    },
    AbiExport {
        function: String,
        params: Vec<String>,
        ret: String,
    },
    DocEmbed {
        content: String,
    },
    GasAdaptive {
        high_gas_ops: u32,
        low_gas_ops: u32,
    },
    Bounty {
        amount: u128,
        condition: String,
    },
    SubExec {
        /// The bytecode hash of the sub-program to execute.
        bytecode_hash: String,
        /// Arguments passed to the sub-program.
        args: Vec<String>,
        /// Maximum gas allowed for this sub-execution.
        gas_limit: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityCodecError {
    UnexpectedEof,
    InvalidUtf8,
    InvalidOpcode(u8),
    TrailingBytes,
    PayloadTooLarge,
}

impl std::fmt::Display for CapabilityCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CapabilityCodecError {}

pub fn encode_capability_payload(
    payload: &CapabilityPayload,
) -> Result<Vec<u8>, CapabilityCodecError> {
    let mut out = Vec::new();
    match payload {
        CapabilityPayload::GpuDispatch {
            kernel,
            args,
            is_simd,
        } => {
            write_string(&mut out, kernel)?;
            write_string_vec(&mut out, args)?;
            write_bool(&mut out, *is_simd);
        }
        CapabilityPayload::Simulate {
            body_ops,
            receipt_slot,
        } => {
            write_u32(&mut out, *body_ops);
            write_string(&mut out, receipt_slot)?;
        }
        CapabilityPayload::ScheduledDispatch {
            period_blocks,
            entry_ops,
        } => {
            write_u32(&mut out, *period_blocks);
            write_u32(&mut out, *entry_ops);
        }
        CapabilityPayload::IntentResolve {
            constraints,
            resolver,
        } => {
            write_string_vec(&mut out, constraints)?;
            write_string(&mut out, resolver)?;
        }
        CapabilityPayload::CrdtOp { kind, key, value } => {
            write_u8(&mut out, *kind);
            write_string(&mut out, key)?;
            write_optional_string(&mut out, value.as_deref())?;
        }
        CapabilityPayload::ProofVerify {
            kind,
            proof,
            input,
            key_or_threshold,
        } => {
            write_u8(&mut out, *kind);
            write_string(&mut out, proof)?;
            write_string(&mut out, input)?;
            write_string(&mut out, key_or_threshold)?;
        }
        CapabilityPayload::StorageOp { kind, data } => {
            write_u8(&mut out, *kind);
            write_string(&mut out, data)?;
        }
        CapabilityPayload::Pathfind {
            from,
            to,
            max_depth,
        } => {
            write_string(&mut out, from)?;
            write_string(&mut out, to)?;
            write_u32(&mut out, *max_depth);
        }
        CapabilityPayload::MempoolScan { max_results } => write_u32(&mut out, *max_results),
        CapabilityPayload::OracleRequest { token, reward } => {
            write_string(&mut out, token)?;
            write_u128(&mut out, *reward);
        }
        CapabilityPayload::EmergencyControl { kind } => write_u8(&mut out, *kind),
        CapabilityPayload::Lifecycle { kind, target } => {
            write_u8(&mut out, *kind);
            write_optional_string(&mut out, target.as_deref())?;
        }
        CapabilityPayload::Serialize { format, data }
        | CapabilityPayload::Deserialize { format, data } => {
            write_u8(&mut out, *format);
            write_string(&mut out, data)?;
        }
        CapabilityPayload::GasEstimate { chain, route } => {
            write_string(&mut out, chain)?;
            write_string(&mut out, route)?;
        }
        CapabilityPayload::ChainMetric { metric } => write_u8(&mut out, *metric),
        CapabilityPayload::EventProvenance { event_type, data } => {
            write_string(&mut out, event_type)?;
            write_string(&mut out, data)?;
        }
        CapabilityPayload::MultiHopSwap { path, amount } => {
            write_string_vec(&mut out, path)?;
            write_u128(&mut out, *amount);
        }
        CapabilityPayload::VectorMath { op, a, b, size } => {
            write_u8(&mut out, *op);
            write_string(&mut out, a)?;
            write_string(&mut out, b)?;
            write_u32(&mut out, *size);
        }
        CapabilityPayload::RoleCheck { role } => write_string(&mut out, role)?,
        CapabilityPayload::MultisigCheck { required, total } => {
            write_u32(&mut out, *required);
            write_u32(&mut out, *total);
        }
        CapabilityPayload::VersionMeta {
            version,
            upgrade_from,
        } => {
            write_string(&mut out, version)?;
            write_optional_string(&mut out, upgrade_from.as_deref())?;
        }
        CapabilityPayload::StorageNamespace { package, key } => {
            write_string(&mut out, package)?;
            write_string(&mut out, key)?;
        }
        CapabilityPayload::AbiExport {
            function,
            params,
            ret,
        } => {
            write_string(&mut out, function)?;
            write_string_vec(&mut out, params)?;
            write_string(&mut out, ret)?;
        }
        CapabilityPayload::DocEmbed { content } => write_string(&mut out, content)?,
        CapabilityPayload::GasAdaptive {
            high_gas_ops,
            low_gas_ops,
        } => {
            write_u32(&mut out, *high_gas_ops);
            write_u32(&mut out, *low_gas_ops);
        }
        CapabilityPayload::Bounty { amount, condition } => {
            write_u128(&mut out, *amount);
            write_string(&mut out, condition)?;
        }
        CapabilityPayload::SubExec {
            bytecode_hash,
            args,
            gas_limit,
        } => {
            write_string(&mut out, bytecode_hash)?;
            write_string_vec(&mut out, args)?;
            write_u64(&mut out, *gas_limit);
        }
    }
    Ok(out)
}

pub fn decode_capability_payload(
    opcode: u8,
    bytes: &[u8],
) -> Result<CapabilityPayload, CapabilityCodecError> {
    let mut reader = Reader { bytes, pos: 0 };
    let payload = match opcode {
        0x80 => CapabilityPayload::GpuDispatch {
            kernel: reader.read_string()?,
            args: reader.read_string_vec()?,
            is_simd: reader.read_bool()?,
        },
        0x81 => CapabilityPayload::Simulate {
            body_ops: reader.read_u32()?,
            receipt_slot: reader.read_string()?,
        },
        0x82 => CapabilityPayload::ScheduledDispatch {
            period_blocks: reader.read_u32()?,
            entry_ops: reader.read_u32()?,
        },
        0x83 => CapabilityPayload::IntentResolve {
            constraints: reader.read_string_vec()?,
            resolver: reader.read_string()?,
        },
        0x84 => CapabilityPayload::CrdtOp {
            kind: reader.read_u8()?,
            key: reader.read_string()?,
            value: reader.read_optional_string()?,
        },
        0x85 => CapabilityPayload::ProofVerify {
            kind: reader.read_u8()?,
            proof: reader.read_string()?,
            input: reader.read_string()?,
            key_or_threshold: reader.read_string()?,
        },
        0x86 => CapabilityPayload::StorageOp {
            kind: reader.read_u8()?,
            data: reader.read_string()?,
        },
        0x87 => CapabilityPayload::Pathfind {
            from: reader.read_string()?,
            to: reader.read_string()?,
            max_depth: reader.read_u32()?,
        },
        0x88 => CapabilityPayload::MempoolScan {
            max_results: reader.read_u32()?,
        },
        0x89 => CapabilityPayload::OracleRequest {
            token: reader.read_string()?,
            reward: reader.read_u128()?,
        },
        0x8A => CapabilityPayload::EmergencyControl {
            kind: reader.read_u8()?,
        },
        0x8B => CapabilityPayload::Lifecycle {
            kind: reader.read_u8()?,
            target: reader.read_optional_string()?,
        },
        0x8C => CapabilityPayload::Serialize {
            format: reader.read_u8()?,
            data: reader.read_string()?,
        },
        0x8D => CapabilityPayload::Deserialize {
            format: reader.read_u8()?,
            data: reader.read_string()?,
        },
        0x8E => CapabilityPayload::GasEstimate {
            chain: reader.read_string()?,
            route: reader.read_string()?,
        },
        0x8F => CapabilityPayload::ChainMetric {
            metric: reader.read_u8()?,
        },
        0x90 => CapabilityPayload::EventProvenance {
            event_type: reader.read_string()?,
            data: reader.read_string()?,
        },
        0x91 => CapabilityPayload::MultiHopSwap {
            path: reader.read_string_vec()?,
            amount: reader.read_u128()?,
        },
        0x92 => CapabilityPayload::VectorMath {
            op: reader.read_u8()?,
            a: reader.read_string()?,
            b: reader.read_string()?,
            size: reader.read_u32()?,
        },
        0x93 => CapabilityPayload::RoleCheck {
            role: reader.read_string()?,
        },
        0x94 => CapabilityPayload::MultisigCheck {
            required: reader.read_u32()?,
            total: reader.read_u32()?,
        },
        0x95 => CapabilityPayload::VersionMeta {
            version: reader.read_string()?,
            upgrade_from: reader.read_optional_string()?,
        },
        0x96 => CapabilityPayload::StorageNamespace {
            package: reader.read_string()?,
            key: reader.read_string()?,
        },
        0x97 => CapabilityPayload::AbiExport {
            function: reader.read_string()?,
            params: reader.read_string_vec()?,
            ret: reader.read_string()?,
        },
        0x98 => CapabilityPayload::DocEmbed {
            content: reader.read_string()?,
        },
        0x99 => CapabilityPayload::GasAdaptive {
            high_gas_ops: reader.read_u32()?,
            low_gas_ops: reader.read_u32()?,
        },
        0x9A => CapabilityPayload::Bounty {
            amount: reader.read_u128()?,
            condition: reader.read_string()?,
        },
        0x9B => CapabilityPayload::SubExec {
            bytecode_hash: reader.read_string()?,
            args: reader.read_string_vec()?,
            gas_limit: reader.read_u64()?,
        },
        _ => return Err(CapabilityCodecError::InvalidOpcode(opcode)),
    };
    if reader.pos != bytes.len() {
        return Err(CapabilityCodecError::TrailingBytes);
    }
    Ok(payload)
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u128(out: &mut Vec<u8>, value: u128) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), CapabilityCodecError> {
    let bytes = value.as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| CapabilityCodecError::PayloadTooLarge)?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

fn write_optional_string(
    out: &mut Vec<u8>,
    value: Option<&str>,
) -> Result<(), CapabilityCodecError> {
    match value {
        Some(value) => {
            write_bool(out, true);
            write_string(out, value)
        }
        None => {
            write_bool(out, false);
            Ok(())
        }
    }
}

fn write_string_vec(out: &mut Vec<u8>, values: &[String]) -> Result<(), CapabilityCodecError> {
    let len = u16::try_from(values.len()).map_err(|_| CapabilityCodecError::PayloadTooLarge)?;
    out.extend_from_slice(&len.to_le_bytes());
    for value in values {
        write_string(out, value)?;
    }
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], CapabilityCodecError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(CapabilityCodecError::UnexpectedEof)?;
        if end > self.bytes.len() {
            return Err(CapabilityCodecError::UnexpectedEof);
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, CapabilityCodecError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_bool(&mut self) -> Result<bool, CapabilityCodecError> {
        Ok(self.read_u8()? != 0)
    }

    fn read_u32(&mut self) -> Result<u32, CapabilityCodecError> {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(self.read_exact(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u128(&mut self) -> Result<u128, CapabilityCodecError> {
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(self.read_exact(16)?);
        Ok(u128::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, CapabilityCodecError> {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(self.read_exact(8)?);
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_string(&mut self) -> Result<String, CapabilityCodecError> {
        let mut len = [0u8; 2];
        len.copy_from_slice(self.read_exact(2)?);
        let len = u16::from_le_bytes(len) as usize;
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(|value| value.to_string())
            .map_err(|_| CapabilityCodecError::InvalidUtf8)
    }

    fn read_optional_string(&mut self) -> Result<Option<String>, CapabilityCodecError> {
        if self.read_bool()? {
            self.read_string().map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_string_vec(&mut self) -> Result<Vec<String>, CapabilityCodecError> {
        let mut len = [0u8; 2];
        len.copy_from_slice(self.read_exact(2)?);
        let len = u16::from_le_bytes(len) as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.read_string()?);
        }
        Ok(values)
    }
}
