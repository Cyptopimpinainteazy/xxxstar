# X3 Chain Browser Extension

Manifest V3 browser extension providing X3 Chain wallet signing, push notifications,
and canonical truth drift detection against the X3 node RPC.

## Build

```
npm install
npm run build       # production bundle -> dist/
npm run dev         # watch mode
```

## Connecting to the X3 node

Set `rpcUrl` in extension storage (default: `http://localhost:9933`).
The background service worker polls `x3_canonicalSnapshot` every 30 seconds.

## Canonical truth drift detection

On each poll, the extension compares three Merkle roots — identity, asset supply,
and treasury vault state — against the previous snapshot.  Any root change raises
a browser notification and queues a `DriftAlert` visible in the popup.

The same root types are defined in `crates/x3-canonical-truth/src/sync.rs` (Rust)
so the desktop app and web portal share identical canonical models.
