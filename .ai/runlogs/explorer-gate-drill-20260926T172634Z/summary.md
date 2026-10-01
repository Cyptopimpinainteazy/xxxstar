# explorer-gate-drill — 2026-09-26T17:26:34Z

Criterion 14 of the public testnet gate, both directions, on the bytes in this tree.

- decoy (plain `python3 -m http.server` on http://127.0.0.1:3410): criterion 14 = FAIL
  `| 14 | Explorer/dashboard | FAIL |`
- apps/explorer (Next.js, `.next` build in the tree): criterion 14 = PASS and named
  `| 14 | Explorer/dashboard | PASS (reached `http://127.0.0.1:3410`, body identifies as the X3 Chain Explorer) |`
