# explorer-gate-drill — 2026-09-26T17:58:09Z

Criterion 14 of the public testnet gate, four directions, on the bytes in this tree.

- decoy (plain `python3 -m http.server` pinned via `X3_EXPLORER_URL` on http://127.0.0.1:3410): FAIL
  `| 14 | Explorer/dashboard | FAIL |`
- matching (apps/explorer reading the same stub RPC as the gate, finalized head #4242):
  PASS, named, and the head it checked is recorded
  `| 14 | Explorer/dashboard | PASS (reached `http://127.0.0.1:3410`, showing finalized head #4242, body identifies as the X3 Chain Explorer) |`
- mismatch (explorer still reading #4242, gate pointed at a stub reporting #9999): FAIL
  `| 14 | Explorer/dashboard | FAIL (reached `http://127.0.0.1:3410`, showing finalized head #4242, body identifies as the X3 Chain Explorer) — it shows #4242 while http://127.0.0.1:3412 reports #9999 |`
- no chain (explorer pointed at an endpoint nothing answers on): PASS, because the page
  renders no height and says the chain is unreachable
  `| 14 | Explorer/dashboard | PASS (reached `http://127.0.0.1:3410`, body identifies as the X3 Chain Explorer) |`
