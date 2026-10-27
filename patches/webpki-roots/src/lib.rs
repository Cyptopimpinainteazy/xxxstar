//! Compatibility shim for dependencies that require the `webpki-roots` 0.25
//! API.
//!
//! Version 0.25.4 bundles trust anchors that include the distrusted emSign
//! roots (RUSTSEC-2026-0381). The current 1.x release removes those roots and
//! exposes the same `TLS_SERVER_ROOTS` field shape used by libp2p. Re-exporting
//! that store keeps the 0.25 dependency boundary without continuing to ship the
//! stale trust bundle.
#![no_std]

/// The 0.25 field names for one trust anchor. The current crate renamed `spki`
/// to `subject_public_key_info`; callers on the 0.25 API still read `spki`.
pub struct TrustAnchor {
    pub subject: &'static [u8],
    pub spki: &'static [u8],
    pub name_constraints: Option<&'static [u8]>,
}

/// A 0.25-compatible view over the current Mozilla trust store.
pub struct TlsServerRoots;

impl TlsServerRoots {
    pub fn iter(&self) -> impl Iterator<Item = TrustAnchor> + '_ {
        webpki_roots_current::TLS_SERVER_ROOTS
            .iter()
            .map(|anchor| TrustAnchor {
                subject: anchor.subject.as_ref(),
                spki: anchor.subject_public_key_info.as_ref(),
                name_constraints: anchor.name_constraints.as_ref().map(|value| value.as_ref()),
            })
    }
}

pub const TLS_SERVER_ROOTS: TlsServerRoots = TlsServerRoots;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distrusted_emsign_ecc_g3_root_is_absent() {
        let needle = b"emSign ECC Root CA - G3";
        let mut roots = 0usize;
        for root in TLS_SERVER_ROOTS.iter() {
            roots += 1;
            assert!(
                !root
                    .subject
                    .windows(needle.len())
                    .any(|window| window == needle),
                "the distrusted emSign ECC Root CA - G3 must not be in the trust store"
            );
        }
        assert!(roots > 0);
    }
}
