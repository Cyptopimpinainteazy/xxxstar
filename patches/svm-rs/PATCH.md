# Solidity compiler installer security patch

Upstream: svm-rs 0.3.5, commit aaffab0d31297d8fb3a9555dce76a37064190fce.
Source copied from the verified local crates.io package; upstream MIT/Apache licenses retained.

The root uses this patch to keep the existing installer API while updating the
Windows ZIP extraction dependency. The same extraction method is exercised on
Linux by deterministic malicious-archive regression fixtures. These tests verify
filesystem containment and permissions; they do not prove Windows compiler execution.

Changes: ZIP 9.0.5; Unix executable mode 0755; stable installer lock inode with explicit non-truncating opens; regression coverage for extraction and lock safety. The standalone dev-test workspace has a separate locked test graph; the root Cargo.lock remains authoritative for production.

Release indexes now select the checksum matching the published artifact path, reject ambiguous or missing published checksums, and retain support for older pathless indexes. Version discovery ignores persistent lock files by listing version directories. Public structs remain unchanged; the builds vector is normalized to published versions in ascending order.

The HTTP client dependency is now reqwest 0.12.28 to retain the public Rustls feature with a maintained TLS stack. This advances the wrapped reqwest::Error type identity; the actual Ethers optional-installer consumer is compile-checked, and downstream consumers supplying version-specific reqwest errors may require migration.
