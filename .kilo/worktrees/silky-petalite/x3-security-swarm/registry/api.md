# Public Threat Registry API

The public threat registry is read-only. It exists to publish cryptographically verifiable protocol facts, not to make off-chain legal accusations.

`GET /v1/threats` returns threat summaries with pagination. `GET /v1/threats/{threat_id}` returns the current status, confidence band, evidence hash, and affected surfaces. `GET /v1/threats/{threat_id}/bundle` returns bundle metadata and storage pointers when public release is allowed.

Entries must omit personal data, unverifiable identity claims, and any statement about off-chain criminal liability. The registry is for deterrence through transparency and for partner systems that want a clean, machine-readable feed.
