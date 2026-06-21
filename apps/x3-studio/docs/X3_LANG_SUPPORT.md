# X3 Language Support

## Status: ✅ FULL

### Syntax Highlighting
Monaco Editor token provider for `.x3` files with:
- Keywords: intent, chain, vm, route, lock, claim, refund, finality, oracle, proof, solver, adapter, relayer, quorum, timeout, slashing, scoreboard, require, emit
- Types: u8-u128, i8-i128, f32, f64, bool, string, Address, Amount, Asset, Hash, Signature, BlockNumber
- Strings, numbers, comments

### Autocomplete
- Keyword completion provider
- Context-sensitive suggestions

### Snippets
- cross-chain intent
- HTLC atomic swap
- cross-VM route
- solver marketplace order
- proof ledger write
- relayer config
- validator task
- slashing rule
- timeout/refund flow
- RPC quorum config
- adapter definition

### Validation
- Brace matching
- require() syntax checking
- Tokenizer-based analysis

### Templates
Cross-chain intent template:
```x3
intent swap {
  chain "source" -> "dest"
  asset 0x... -> 0x...
  amount 1000
  deadline block+100
  solver any
}
```

HTLC template:
```x3
htlc {
  sender: 0x...
  receiver: 0x...
  secret_hash: 0x...
  timeout: block + 100
  amount: 1000
}
```
