# X3 Release Beacons

This component closes the authority boundary for cross-domain settlement
ordering. Each genesis validator runs one `publish` process with a dedicated
Ed25519 key. The process reads X3's GRANDPA-finalized head, signs a statement
binding the attestor id, block height and block hash, and submits that
attestation to the exchange.

The exchange is an evidence transport. It only admits keys in its configured
authority set, verifies every signature, removes a validator's prior evidence
if it signs a conflicting hash at the same height, and durably persists the
accepted set. The coordinator still applies its own threshold policy before it
can order a settlement release.

## Exchange

```bash
x3-release-beacon exchange \
  --listen 127.0.0.1:9750 \
  --store /var/lib/x3/release-beacons.json \
  --authorized-key <validator-1-ed25519-public-key> \
  --authorized-key <validator-2-ed25519-public-key>
```

Run the exchange behind an authenticated TLS reverse proxy. Attestations are
public evidence, but unauthenticated exposure still permits resource abuse.

## Validator Publisher

```bash
export X3_RELEASE_BEACON_PRIVATE_KEY_HEX=<32-byte-validator-key>
x3-release-beacon publish \
  --rpc-url http://127.0.0.1:9944 \
  --exchange-url https://beacons.example.net \
  --attestor-id validator-1 \
  --poll-ms 2000
```

Use a dedicated signing key, not Aura, GRANDPA, treasury, gateway or account
keys. The public key configured at the exchange must match the key held by the
publisher. A missing, unauthorized or malformed key fails closed.

## HTTP Contract

`POST /v1/x3-release-beacons` accepts one signed attestation.

`POST /v1/x3-release-beacons/query` accepts an exact height/hash pair and returns
all verified attestations for it. The endpoint never claims that threshold was
met; that decision belongs to the coordinator's policy.
