# Solana pubsub legacy transport security patch

Exact crates.io Rust sources are retained under vendor/ with upstream Apache license. Only manifest source paths and both websocket TLS features change: native-tls replaces the legacy Rustls backend. Public Rust APIs remain unchanged. Native platform certificate validation is preserved and verified using real loopback HTTPS/WSS tests; no post-quantum TLS capability is claimed. Live chain integration is a separate gate. Provenance: patches/solana-tls-provenance.json.
