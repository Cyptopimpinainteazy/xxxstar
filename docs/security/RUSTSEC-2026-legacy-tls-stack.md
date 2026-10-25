# Residual legacy TLS advisories

## Advisories

- `RUSTSEC-2026-0346`: rustls client-session `Debug` output can expose TLS
  secrets.
- `RUSTSEC-2026-0349`: rustls resumption can be downgraded from a hybrid
  post-quantum key exchange to a classical one.
- `RUSTSEC-2026-0381`: older bundled `webpki-roots` trust stores can accept
  distrusted emSign intermediates.
- `RUSTSEC-2026-0382`: older `rustls-webpki` verification can accept the same
  distrusted intermediates.

## Reachable copies

`cargo deny check advisories` attributes the remaining copies to legacy
Ethers 2 / Solana 3 networking clients, plus dev/test Solana program crates.
The main X3 node TLS path resolves `rustls 0.23.46`,
`rustls-webpki 0.103.16`, and the fixed `webpki-root-certs 1.0.10` store.

## Why these are accepted

The `Debug` disclosure requires a custom `ClientSessionStore` that formats
private session values; no X3 client implements one. The ServerHello prediction
issue affects session resumption with hybrid post-quantum groups; the remaining
rustls 0.21 clients do not configure such a group. The emSign finding requires a
certificate path through those compromised intermediates, and the affected root
stores are confined to legacy client/test code rather than the node verifier.

These are compatibility debts, not waived hard failures. They must be removed
before legacy client code is allowed to accept arbitrary peer URLs or execute in
a value-moving mainnet process.

## Migration trigger

Upgrade or replace Ethers 2, Solana 3 websocket clients, and their dev/test
counterparts so every lockfile resolves at least:

- rustls `0.23.46`
- rustls-webpki `0.103.16`
- webpki-roots `1.0.10`

Remove these records and their ignore entries when the vulnerable versions no
longer appear in any lockfile.
