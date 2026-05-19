# X3 Opcode Table v0.1

Detailed opcode table for the X3 VM. The opcode map is fixed and must not be mutated without a version bump.

Format: 0x?? MNEMONIC  -- operands / description -- flags -- gas

Arithmetic and Logical (0x01..0x1F)
- 0x01 ADD_RRR  R[a],R[b],R[c]        -- ra = rb + rc                         -- REG3 -- 1
- 0x02 SUB_RRR  R[a],R[b],R[c]        -- ra = rb - rc                         -- REG3 -- 1
- 0x03 MUL_RRR  R[a],R[b],R[c]        -- ra = rb * rc                         -- REG3 -- 2
- 0x04 DIV_RRR  R[a],R[b],R[c]        -- ra = rb / rc                         -- REG3 -- 3
- 0x05 MOD_RRR  R[a],R[b],R[c]        -- ra = rb % rc                         -- REG3 -- 3
- 0x06 NEG_RR   R[a],R[b]             -- ra = -rb                             -- REG2 -- 1
- 0x07 AND_RRR  R[a],R[b],R[c]        -- ra = rb & rc                         -- REG3 -- 1
- 0x08 OR_RRR   R[a],R[b],R[c]        -- ra = rb | rc                         -- REG3 -- 1
- 0x09 XOR_RRR  R[a],R[b],R[c]        -- ra = rb ^ rc                         -- REG3 -- 1
- 0x0A SHL_RRR  R[a],R[b],R[c]        -- ra = rb << rc (logical)              -- REG3 -- 1
- 0x0B SHR_RRR  R[a],R[b],R[c]        -- ra = rb >> rc (logical)              -- REG3 -- 1
- 0x0C NOT_RR   R[a],R[b]             -- ra = !rb (logical not)               -- REG2 -- 1
- 0x0D CMP_RRR  R[a],R[b],R[c]        -- ra = cmp(rb,rc) three-way (-1,0,1)   -- REG3 -- 1
- 0x0E ABS_RR   R[a],R[b]             -- ra = abs(rb)                         -- REG2 -- 1
- 0x0F CLZ_RR   R[a],R[b]             -- ra = count leading zeros              -- REG2 -- 1

Memory & Stack (0x10..0x2F)
- 0x10 LOAD_RAI R[a], [R[b] + imm16]  -- memory load 128-bit               -- REGREGIMM -- 5
- 0x11 STORE_RAI R[a], [R[b] + imm16] -- memory store 128-bit              -- REGREGIMM -- 5
- 0x12 LOAD8_RAI R[a], [R[b] + imm16] -- load 8-bit                          -- REGREGIMM -- 3
- 0x13 STORE8_RAI R[a], [R[b] + imm16]-- store 8-bit                         -- REGREGIMM -- 3
- 0x14 PUSH_IMM imm16                 -- push signed immediate to operand stack -- IMM -- 1
- 0x15 POP reg                        -- pop stack into register              -- REG -- 1
- 0x16 DUP                            -- duplicate top-of-stack               -- STACK -- 1
- 0x17 SWAP                           -- swap top two entries on stack        -- STACK -- 1
- 0x18 TGET idx                       -- stack get element idx into R0        -- STACKIMM -- 2

Control Flow (0x30..0x4F)
- 0x30 JMP offset16                   -- pc = pc + offset16                   -- BR -- 2
- 0x31 JZ offset16                    -- jump if zero top-of-stack            -- BRCOND -- 2
- 0x32 JNZ offset16                   -- jump if not zero                     -- BRCOND -- 2
- 0x33 CALL addr16                    -- call function at addr16              -- CALL -- 5
- 0x34 RET                            -- return from call                     -- RET -- 5
- 0x35 SWITCH table_idx               -- jump by table                       -- BR_SWITCH -- 3
- 0x36 LOOP_BEGIN                     -- mark start of loop (for verifier)    -- LOOP -- 1
- 0x37 LOOP_END                       -- end loop                              -- LOOP -- 1

Crypto and Signatures (0x40..0x5F)
- 0x40 CRYPTO_SHA256 R[a], R[b]       -- r[a] = sha256(reg[b])               -- CRYPTO -- 10
- 0x41 CRYPTO_KECCAK R[a], R[b]       -- r[a] = keccak256(reg[b])            -- CRYPTO -- 10
- 0x42 CRYPTO_BLAKE3 R[a], R[b]       -- r[a] = blake3(reg[b])               -- CRYPTO -- 8
- 0x43 SIG_VERIFY_SECP256K1 R[a], R[b], R[c] -- verify signature              -- CRYPTO -- 50
- 0x44 SIG_VERIFY_ED25519 R[a], R[b], R[c]  -- verify ed25519 signature        -- CRYPTO -- 50

VM-native and Host (0x60..0x7F)
- 0x60 EVM_CALL R[a], addr16          -- bridge call to EVM adapter           -- HOSTCALL -- 100
- 0x61 SVM_CALL R[a], addr16          -- bridge call to SVM adapter           -- HOSTCALL -- 100
- 0x62 EVM_RETURN R[a]                -- return value from EVM call           -- HOSTRET -- 5
- 0x63 SVM_RETURN R[a]                -- return value from SVM call           -- HOSTRET -- 5
- 0x64 ATOMIC_BEGIN                   -- begin atomic window                  -- ATOMIC -- 250
- 0x65 ATOMIC_COMMIT                  -- commit atomic window                 -- ATOMIC -- 500
- 0x66 ATOMIC_ROLLBACK                -- rollback atomic window               -- ATOMIC -- 250

Vector/SIMD (0x80..0x9F)
- 0x80 SIMD_ADD V[a], V[b], V[c]      -- vector add 128-bit                   -- VEC -- 2
- 0x81 SIMD_SUB V[a], V[b], V[c]      -- vector sub                           -- VEC -- 2
- 0x82 SIMD_MUL V[a], V[b], V[c]      -- vector multiply                      -- VEC -- 3
- 0x83 SIMD_SHL V[a], V[b], imm8      -- vector shift left                    -- VEC -- 1
- 0x84 SIMD_SHR V[a], V[b], imm8      -- vector shift right                   -- VEC -- 1

Misc & System (0xA0..0xBF)
- 0xA0 NOP                            -- no-op                              -- 0
- 0xA1 HALT                           -- halt VM                           -- 0
- 0xA2 TRACE R[a]                     -- trace/log output to debug sink     -- SYS -- 0
- 0xA3 EXPLORE_HOTPATH hint16         -- increment hot counter hint         -- SYS -- 0
- 0xA4 GROW_MEMORY pages16            -- grow linear memory                 -- SYS -- 10 per page
- 0xA5 SHRINK_MEMORY pages16          -- shrink linear memory               -- SYS -- 10 per page

Reserved opcodes: 0xC0..0xFE - experimental or JIT hints

Notes
- All opcodes must be validated by the verifier, including operand sizes and flags.
- Opcode encodings and flags are fixed; any change requires version bump.
