//! The packet wire format must fail closed on damaged input.
//!
//! `Packet::from_wire_format` is the comit path's parser. `pallets/x3-kernel`'s packet adapters call
//! it on bytes that arrive inside a transaction, and between a malformed packet and the per-domain
//! decoder sit three gates: the header fields, a payload-size check against the bytes actually
//! present, and a CRC32. This file damages packets built by the crate's own `to_wire_format` and
//! asserts what each gate does — that a truncation is refused, that no single flipped byte panics,
//! and that the one gate that is easy to get wrong (a payload claiming a huge vector) errors instead
//! of allocating.

#![cfg(feature = "std")]

use crc32fast::Hasher;
use parity_scale_codec::Encode;
use x3_packet_schema::{EvmPacket, Packet, PacketHeader, SvmPacket, X3VmPacket, U256};

fn fixtures() -> Vec<(&'static str, Packet)> {
    vec![
        (
            "evm call",
            Packet::Evm(EvmPacket::Call {
                contract: [0x42; 20],
                function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
                args: vec![1, 2, 3, 4, 5, 6, 7, 8],
                value: U256::from(7u64),
            }),
        ),
        (
            "svm invoke",
            Packet::Svm(SvmPacket::Invoke {
                program_id: [0x11; 32],
                accounts: vec![],
                data: vec![9, 9, 9],
            }),
        ),
        (
            "x3vm transfer",
            Packet::X3Vm(X3VmPacket::Transfer {
                from_domain: 0,
                to_domain: 1,
                asset_id: 0,
                amount: 1_000u128,
                recipient: vec![0x33; 32],
            }),
        ),
    ]
}

#[test]
fn the_fixtures_round_trip_through_the_wire_format() {
    // Without this, every sweep below could be passing because the fixture never encoded.
    for (name, packet) in fixtures() {
        let bytes = packet.to_wire_format().expect("a valid packet must encode");
        let decoded = Packet::from_wire_format(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(decoded, packet, "{name} must survive the round trip");
    }
}

#[test]
fn every_truncation_is_refused() {
    for (name, packet) in fixtures() {
        let bytes = packet.to_wire_format().unwrap();
        for len in 0..bytes.len() {
            assert!(
                Packet::from_wire_format(&bytes[..len]).is_err(),
                "{name}: a {len}-byte prefix of a valid packet decoded"
            );
        }
    }
}

#[test]
fn no_single_byte_mutation_panics_the_parser() {
    for (name, packet) in fixtures() {
        let bytes = packet.to_wire_format().unwrap();
        for offset in 0..bytes.len() {
            for value in [0x00u8, 0x01, 0x7F, 0x80, 0xFF] {
                let mut damaged = bytes.clone();
                if damaged[offset] == value {
                    continue;
                }
                damaged[offset] = value;
                // The parser must return, not abort. If it accepts the bytes the packet must at
                // least be one it can serialise again.
                if let Ok(parsed) = Packet::from_wire_format(&damaged) {
                    let _ = parsed.to_wire_format();
                }
                let _ = name;
            }
        }
    }
}

#[test]
fn the_header_gates_refuse_by_name() {
    let packet = fixtures()[0].1.clone();
    let bytes = packet.to_wire_format().unwrap();

    // version is the first field of the SCALE-encoded header.
    let mut bad_version = bytes.clone();
    bad_version[0] = 2;
    assert_eq!(
        Packet::from_wire_format(&bad_version),
        Err("Invalid packet version")
    );

    // domain_mask is the second.
    let mut no_domain = bytes.clone();
    no_domain[1] = 0;
    assert_eq!(
        Packet::from_wire_format(&no_domain),
        Err("Must target at least one domain")
    );

    // payload_size is a u16 at offset 4; 0xFFFF is larger than the bytes present.
    let mut oversized = bytes.clone();
    oversized[4..6].copy_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(
        Packet::from_wire_format(&oversized),
        Err("Packet too short for payload + CRC")
    );
}

#[test]
fn a_damaged_body_is_caught_by_the_checksum() {
    let packet = fixtures()[0].1.clone();
    let bytes = packet.to_wire_format().unwrap();
    // One byte of the payload, past the 26-byte header and the type byte.
    let mut damaged = bytes.clone();
    let last_payload = damaged.len() - 5;
    damaged[last_payload] ^= 0xFF;
    assert_eq!(Packet::from_wire_format(&damaged), Err("CRC32 mismatch"));

    // And the type byte, which the checksum covers too.
    let mut bad_type = bytes.clone();
    bad_type[26] = 9;
    assert_eq!(Packet::from_wire_format(&bad_type), Err("CRC32 mismatch"));
}

#[test]
fn a_payload_claiming_a_huge_vector_errors_instead_of_allocating() {
    // The CRC is the only thing between the header and `EvmPacket::decode`, so a crafted payload
    // needs a *correct* checksum to reach the decoder — which is what this builds. `Call`'s SCALE
    // layout is contract(20) | selector(4) | args(Vec<u8>) | value(U256); the `args` length is
    // compact-encoded, and `0x03 FF FF FF FF` is the four-byte compact form of u32::MAX.
    const HUGE_LEN: [u8; 5] = [0x03, 0xFF, 0xFF, 0xFF, 0xFF];

    let mut payload = Vec::new();
    payload.extend_from_slice(&[0x42; 20]);
    payload.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
    payload.extend_from_slice(&HUGE_LEN);
    payload.extend_from_slice(&[0u8; 32]); // a plausible `value`, never reached

    let header = PacketHeader::new(1, 0b0001, payload.len() as u16);
    let header_bytes = header.encode();
    let type_byte = 0u8;
    let mut hasher = Hasher::new();
    hasher.update(&header_bytes);
    hasher.update(&[type_byte]);
    hasher.update(&payload);
    let crc = hasher.finalize();

    let mut wire = header_bytes;
    wire.push(type_byte);
    wire.extend_from_slice(&payload);
    wire.extend_from_slice(&crc.to_le_bytes());

    match Packet::from_wire_format(&wire) {
        Err("Failed to decode EVM packet") => {}
        other => panic!("a 4-billion-element vector must be refused, got {other:?}"),
    }
}
