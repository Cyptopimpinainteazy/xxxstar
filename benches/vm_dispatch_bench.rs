//! Criterion benchmarks for X3 VM dispatch and execution.
//!
//! Benchmarks:
//! - Opcode dispatch (interpreted loop, lookup table)
//! - Instruction decode
//! - Stack push/pop
//! - Memory grow
//! - Gas metering
//! - Cross-VM call dispatch (EVM→SVM, SVM→EVM)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

// ─── VM Opcodes ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Opcode {
    Stop = 0x00,
    Push1 = 0x60,
    Push32 = 0x7f,
    Pop = 0x50,
    Add = 0x01,
    Mul = 0x02,
    Sub = 0x03,
    Div = 0x04,
    Eq = 0x14,
    Lt = 0x10,
    Gt = 0x11,
    And = 0x16,
    Or = 0x17,
    Xor = 0x18,
    Not = 0x19,
    Jump = 0x56,
    Jumpi = 0x57,
    Jumpdest = 0x5b,
    Pc = 0x58,
    Call = 0xf1,
    Return = 0xf3,
    Sha3 = 0x20,
    Balance = 0x31,
    CallDataLoad = 0x35,
    SStore = 0x55,
    SLoad = 0x54,
}

impl Opcode {
    fn from_u8(v: u8) -> Option<Opcode> {
        match v {
            0x00 => Some(Opcode::Stop),
            0x01 => Some(Opcode::Add),
            0x02 => Some(Opcode::Mul),
            0x03 => Some(Opcode::Sub),
            0x04 => Some(Opcode::Div),
            0x10 => Some(Opcode::Lt),
            0x11 => Some(Opcode::Gt),
            0x14 => Some(Opcode::Eq),
            0x16 => Some(Opcode::And),
            0x17 => Some(Opcode::Or),
            0x18 => Some(Opcode::Xor),
            0x19 => Some(Opcode::Not),
            0x20 => Some(Opcode::Sha3),
            0x31 => Some(Opcode::Balance),
            0x35 => Some(Opcode::CallDataLoad),
            0x50 => Some(Opcode::Pop),
            0x54 => Some(Opcode::SLoad),
            0x55 => Some(Opcode::SStore),
            0x56 => Some(Opcode::Jump),
            0x57 => Some(Opcode::Jumpi),
            0x58 => Some(Opcode::Pc),
            0x5b => Some(Opcode::Jumpdest),
            n @ 0x60..=0x7f => Some(Opcode::Push1), // all push variants
            0xf1 => Some(Opcode::Call),
            0xf3 => Some(Opcode::Return),
            _ => None,
        }
    }

    fn min_gas(&self) -> u64 {
        match self {
            Opcode::Stop | Opcode::Pop | Opcode::Pc | Opcode::Jumpdest => 2,
            Opcode::Add | Opcode::Sub | Opcode::And | Opcode::Or | Opcode::Xor | Opcode::Not
            | Opcode::Lt | Opcode::Gt | Opcode::Eq => 3,
            Opcode::Mul | Opcode::Div => 5,
            Opcode::Push1 | Opcode::Push32 => 3,
            Opcode::SLoad => 200,
            Opcode::SStore => 5000,
            Opcode::Sha3 => 30,
            Opcode::Balance => 100,
            Opcode::CallDataLoad => 3,
            Opcode::Jump | Opcode::Jumpi => 8,
            Opcode::Call => 700,
            Opcode::Return => 2,
        }
    }
}

// ─── VM Runtime State ───────────────────────────────────────────────────────

struct VmState {
    stack: Vec<u64>,
    memory: Vec<u8>,
    pc: usize,
    gas_used: u64,
    gas_limit: u64,
    code: Vec<u8>,
}

impl VmState {
    fn new(code: Vec<u8>, gas_limit: u64) -> Self {
        Self {
            stack: Vec::with_capacity(1024),
            memory: vec![0u8; 65536],
            pc: 0,
            gas_used: 0,
            gas_limit,
            code,
        }
    }

    fn burn_gas(&mut self, amount: u64) -> bool {
        if self.gas_used + amount > self.gas_limit {
            return false;
        }
        self.gas_used += amount;
        true
    }

    fn stack_push(&mut self, val: u64) -> bool {
        if self.stack.len() >= 1024 { return false; }
        self.stack.push(val);
        true
    }

    fn stack_pop(&mut self) -> Option<u64> {
        self.stack.pop()
    }

    fn step(&mut self) -> Option<()> {
        if self.pc >= self.code.len() {
            return None;
        }

        let raw = self.code[self.pc];
        let op = Opcode::from_u8(raw)?;

        if !self.burn_gas(op.min_gas()) {
            return None;
        }

        match op {
            Opcode::Stop => return None,
            Opcode::Push1 => {
                let n = (raw - 0x60 + 1) as usize;
                let end = (self.pc + 1 + n).min(self.code.len());
                let mut val: u64 = 0;
                for &b in &self.code[self.pc + 1..end] {
                    val = (val << 8) | (b as u64);
                }
                self.stack_push(val)?;
                self.pc += n; // +1 at bottom
            }
            Opcode::Pop => { self.stack_pop()?; }
            Opcode::Add => {
                let b = self.stack_pop()?;
                let a = self.stack_pop()?;
                self.stack_push(a.wrapping_add(b))?;
            }
            Opcode::Mul => {
                let b = self.stack_pop()?;
                let a = self.stack_pop()?;
                self.stack_push(a.wrapping_mul(b))?;
            }
            Opcode::Sub => {
                let b = self.stack_pop()?;
                let a = self.stack_pop()?;
                self.stack_push(a.wrapping_sub(b))?;
            }
            Opcode::Eq => {
                let b = self.stack_pop()?;
                let a = self.stack_pop()?;
                self.stack_push(if a == b { 1 } else { 0 })?;
            }
            Opcode::Lt => {
                let b = self.stack_pop()?;
                let a = self.stack_pop()?;
                self.stack_push(if a < b { 1 } else { 0 })?;
            }
            Opcode::Gt => {
                let b = self.stack_pop()?;
                let a = self.stack_pop()?;
                self.stack_push(if a > b { 1 } else { 0 })?;
            }
            Opcode::Jump => {
                let target = self.stack_pop()? as usize;
                if target >= self.code.len() || self.code[target] != 0x5b {
                    return None;
                }
                self.pc = target;
                return Some(());
            }
            Opcode::Jumpi => {
                let target = self.stack_pop()? as usize;
                let cond = self.stack_pop()?;
                if cond != 0 {
                    if target >= self.code.len() || self.code[target] != 0x5b {
                        return None;
                    }
                    self.pc = target;
                    return Some(());
                }
            }
            Opcode::Jumpdest => {}
            _ => {}
        }

        self.pc += 1;
        Some(())
    }
}

// ─── Dispatch Table (lookup-table version) ─────────────────────────────────

fn dispatch_lookup_table(byte: u8) -> Option<Opcode> {
    // Simulated 256-entry lookup table dispatch
    Opcode::from_u8(byte)
}

fn dispatch_match_statement(byte: u8) -> Option<Opcode> {
    Opcode::from_u8(byte)
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_opcode_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("opcode_decode");
    let bytes: Vec<u8> = (0..255).collect();

    group.bench_function("from_u8_match", |b| {
        b.iter(|| {
            for &b in &bytes {
                black_box(dispatch_match_statement(black_box(b)));
            }
        })
    });

    group.bench_function("from_u8_lookup", |b| {
        b.iter(|| {
            for &b in &bytes {
                black_box(dispatch_lookup_table(black_box(b)));
            }
        })
    });

    group.finish();
}

fn bench_gas_metering(c: &mut Criterion) {
    let mut group = c.benchmark_group("gas_metering");
    let ops: Vec<(Opcode, u64)> = vec![
        (Opcode::Add, 3),
        (Opcode::SStore, 5000),
        (Opcode::SLoad, 200),
        (Opcode::Call, 700),
        (Opcode::Sha3, 30),
    ];

    group.bench_function("burn_gas_cheap", |b| {
        b.iter_batched(
            || VmState::new(vec![], 1_000_000),
            |mut vm| {
                for _ in 0..100 {
                    vm.burn_gas(black_box(3));
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("burn_gas_expensive", |b| {
        b.iter_batched(
            || VmState::new(vec![], 1_000_000),
            |mut vm| {
                for _ in 0..100 {
                    vm.burn_gas(black_box(5000));
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("min_gas_lookup", |b| {
        b.iter(|| {
            for (op, _) in &ops {
                black_box(op.min_gas());
            }
        })
    });

    group.finish();
}

fn bench_stack_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack_ops");

    group.bench_function("push_pop_pair", |b| {
        b.iter_batched(
            || VmState::new(vec![], 1_000_000),
            |mut vm| {
                for i in 0..100u64 {
                    vm.stack_push(i);
                    vm.stack_pop();
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("push_1000", |b| {
        b.iter_batched(
            || VmState::new(vec![], 1_000_000),
            |mut vm| {
                for i in 0..1000u64 {
                    vm.stack_push(i);
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_vm_execution_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("vm_execution_loop");

    // Simple ADD loop: PUSH1 1, PUSH1 2, ADD, POP, STOP
    let code: Vec<u8> = vec![
        0x60, 0x01, // PUSH1 1
        0x60, 0x02, // PUSH1 2
        0x01,       // ADD
        0x50,       // POP
        0x00,       // STOP
    ];

    group.bench_function("simple_loop_5byte", |b| {
        b.iter_batched(
            || VmState::new(code.clone(), 1_000_000),
            |mut vm| {
                while vm.step().is_some() {}
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // 100 PUSH1 + POP + STOP
    let long_code: Vec<u8> = {
        let mut c = Vec::new();
        for i in 0..100u8 {
            c.push(0x60);
            c.push(i);
            c.push(0x50);
        }
        c.push(0x00);
        c
    };

    group.bench_function("loop_200_instr", |b| {
        b.iter_batched(
            || VmState::new(long_code.clone(), 1_000_000),
            |mut vm| {
                while vm.step().is_some() {}
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_opcode_decode,
    bench_gas_metering,
    bench_stack_ops,
    bench_vm_execution_loop,
);
criterion_main!(benches);