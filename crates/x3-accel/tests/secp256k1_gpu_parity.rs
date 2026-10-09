//! GPU secp256k1 verification must agree with `CpuBackend` on every job.
//!
//! Runs only with `--features wgpu`. Without a GPU adapter it skips visibly,
//! or fails when `X3_REQUIRE_GPU=1`.
#![cfg(feature = "wgpu")]

use num_bigint::BigUint;
use secp256k1::{Message, PublicKey, Scalar, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use x3_accel::{secp256k1_with_parity, AccelBackend, CpuBackend, Secp256k1VerifyJob, WgpuBackend};

fn n() -> BigUint {
    BigUint::parse_bytes(
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
        16,
    )
    .unwrap()
}

fn p() -> BigUint {
    BigUint::parse_bytes(
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F",
        16,
    )
    .unwrap()
}

fn be32(v: &BigUint) -> [u8; 32] {
    let bytes = v.to_bytes_be();
    let mut out = [0u8; 32];
    out[32 - bytes.len()..].copy_from_slice(&bytes);
    out
}

fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for part in parts {
        h.update(part);
    }
    h.finalize().into()
}

struct Case {
    label: String,
    job: Secp256k1VerifyJob,
}

fn job(
    label: impl Into<String>,
    message_hash: [u8; 32],
    signature: [u8; 64],
    public_key: Vec<u8>,
) -> Case {
    Case {
        label: label.into(),
        job: Secp256k1VerifyJob {
            message_hash,
            signature,
            public_key,
        },
    }
}

/// A signature whose R has x >= n, so r = x(R) - n and only the `r + n`
/// comparison accepts it. Random signatures hit this with probability ~2^-128,
/// so it is built backwards: pick R, s, z, then solve for Q = r^-1(sR - zG).
fn r_plus_n_case(secp: &Secp256k1<secp256k1::All>) -> (Case, Case) {
    let (p, n) = (p(), n());
    // Start above n: x = n would give r = 0, which is rejected before any curve math.
    let mut x = &n + 1u32;
    let (x_r, y_r) = loop {
        let rhs = (x.modpow(&BigUint::from(3u32), &p) + 7u32) % &p;
        let y = rhs.modpow(&((&p + 1u32) / 4u32), &p);
        if (&y * &y) % &p == rhs {
            break (x, y);
        }
        x += 1u32;
    };
    let mut r_point = vec![4u8];
    r_point.extend(be32(&x_r));
    r_point.extend(be32(&y_r));
    let r_point = PublicKey::from_slice(&r_point).unwrap();
    let r = &x_r - &n;
    let s = BigUint::from(0x1234_5678u32);
    let z = hash(&[b"r+n edge"]);
    let s_r = r_point
        .mul_tweak(secp, &Scalar::from_be_bytes(be32(&s)).unwrap())
        .unwrap();
    let z_g = PublicKey::from_secret_key(secp, &SecretKey::from_slice(&z).unwrap());
    let r_inv = r.modpow(&(&n - 2u32), &n);
    let q = s_r
        .combine(&z_g.negate(secp))
        .unwrap()
        .mul_tweak(secp, &Scalar::from_be_bytes(be32(&r_inv)).unwrap())
        .unwrap();
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&be32(&r));
    sig[32..].copy_from_slice(&be32(&s));
    // The unreduced x(R) as r is >= n and must be rejected at parse time.
    let mut unreduced = sig;
    unreduced[..32].copy_from_slice(&be32(&x_r));
    (
        job(
            "r+n: x(R) >= n, r = x(R) - n",
            z,
            sig,
            q.serialize().to_vec(),
        ),
        job(
            "r+n: unreduced r = x(R) >= n",
            z,
            unreduced,
            q.serialize().to_vec(),
        ),
    )
}

fn cases() -> Vec<Case> {
    let secp = Secp256k1::new();
    let n = n();
    let mut out = Vec::new();
    for i in 0u32..1500 {
        let seed = i.to_le_bytes();
        let sk = SecretKey::from_slice(&hash(&[b"key", &seed])).unwrap();
        let pk = PublicKey::from_secret_key(&secp, &sk);
        // A few hashes >= n (z must be reduced) and one all-zero hash.
        let z = match i % 50 {
            0 => [0xff; 32],
            1 => [0u8; 32],
            _ => hash(&[b"msg", &seed]),
        };
        let sig = secp
            .sign_ecdsa(&Message::from_digest(z), &sk)
            .serialize_compact();
        let compressed = pk.serialize().to_vec();
        let uncompressed = pk.serialize_uncompressed().to_vec();
        out.push(job(
            format!("{i} valid compressed"),
            z,
            sig,
            compressed.clone(),
        ));
        if i % 3 == 0 {
            out.push(job(
                format!("{i} valid uncompressed"),
                z,
                sig,
                uncompressed.clone(),
            ));
        }
        if i % 5 == 0 {
            // Hybrid encoding: 0x06/0x07 carries y and its parity.
            let mut hybrid = uncompressed.clone();
            hybrid[0] = 6 | (uncompressed[64] & 1);
            out.push(job(
                format!("{i} hybrid good parity"),
                z,
                sig,
                hybrid.clone(),
            ));
            hybrid[0] ^= 1;
            out.push(job(format!("{i} hybrid wrong parity"), z, sig, hybrid));
        }
        let mut flipped = z;
        flipped[(i as usize) % 32] ^= 1 << (i % 8);
        out.push(job(
            format!("{i} wrong message"),
            flipped,
            sig,
            compressed.clone(),
        ));
        if i % 2 == 0 {
            // High-S twin of a valid signature: same curve equation, rejected by policy.
            let s = BigUint::from_bytes_be(&sig[32..]);
            let mut high = sig;
            high[32..].copy_from_slice(&be32(&(&n - &s)));
            out.push(job(format!("{i} high-S"), z, high, compressed.clone()));
        }
        if i % 4 == 0 {
            let mut bumped = sig;
            bumped[31] = bumped[31].wrapping_add(1);
            out.push(job(format!("{i} r+1"), z, bumped, compressed.clone()));
            let other = PublicKey::from_secret_key(
                &secp,
                &SecretKey::from_slice(&hash(&[b"other", &seed])).unwrap(),
            );
            out.push(job(
                format!("{i} wrong key"),
                z,
                sig,
                other.serialize().to_vec(),
            ));
            let mut off_curve = uncompressed.clone();
            off_curve[64] ^= 1;
            out.push(job(format!("{i} off-curve key"), z, sig, off_curve));
        }
        if i % 25 == 0 {
            let mut zero_r = sig;
            zero_r[..32].fill(0);
            out.push(job(format!("{i} r = 0"), z, zero_r, compressed.clone()));
            let mut zero_s = sig;
            zero_s[32..].fill(0);
            out.push(job(format!("{i} s = 0"), z, zero_s, compressed.clone()));
            let mut r_n = sig;
            r_n[..32].copy_from_slice(&be32(&n));
            out.push(job(format!("{i} r = n"), z, r_n, compressed.clone()));
            let mut s_n = sig;
            s_n[32..].copy_from_slice(&be32(&n));
            out.push(job(format!("{i} s = n"), z, s_n, compressed.clone()));
            let mut bad_prefix = compressed.clone();
            bad_prefix[0] = 5;
            out.push(job(format!("{i} bad prefix"), z, sig, bad_prefix));
            out.push(job(
                format!("{i} 64-byte key"),
                z,
                sig,
                uncompressed[1..].to_vec(),
            ));
            out.push(job(format!("{i} empty key"), z, sig, Vec::new()));
            let mut x_ge_p = vec![2u8];
            x_ge_p.extend(be32(&(p() + 1u32 - 1u32)));
            out.push(job(format!("{i} x = p"), z, sig, x_ge_p));
        }
        // Random bytes: almost always parse failures or invalid.
        let junk = hash(&[b"junk", &seed]);
        let mut junk_sig = [0u8; 64];
        junk_sig[..32].copy_from_slice(&junk);
        junk_sig[32..].copy_from_slice(&hash(&[&junk]));
        let mut junk_key = vec![2 | (junk[0] & 1)];
        junk_key.extend(hash(&[b"jk", &junk]));
        out.push(job(format!("{i} random bytes"), junk, junk_sig, junk_key));
    }
    // Q = G and Q = -G: G + Q is a doubling and the point at infinity. With
    // the GLV split, Q = ±λG makes λG and Q coincide or cancel in the table.
    let lambda = BigUint::parse_bytes(
        b"5363ad4cc05c30e0a5261c028812645a122e22ea20816678df02967c1b23bd72",
        16,
    )
    .unwrap();
    for (label, scalar) in [
        ("Q = G", BigUint::from(1u32)),
        ("Q = -G", &n - 1u32),
        ("Q = 2G", BigUint::from(2u32)),
        ("Q = lambda G", lambda.clone()),
        ("Q = -lambda G", &n - &lambda),
    ] {
        let sk = SecretKey::from_slice(&be32(&scalar)).unwrap();
        for k in 0u8..8 {
            let z = hash(&[label.as_bytes(), &[k]]);
            let sig = secp
                .sign_ecdsa(&Message::from_digest(z), &sk)
                .serialize_compact();
            out.push(job(
                format!("{label} #{k}"),
                z,
                sig,
                PublicKey::from_secret_key(&secp, &sk).serialize().to_vec(),
            ));
        }
    }
    let (accepted, unreduced) = r_plus_n_case(&secp);
    out.push(accepted);
    out.push(unreduced);
    out
}

#[test]
fn gpu_secp256k1_matches_cpu_on_every_job() {
    let gpu = match WgpuBackend::try_new() {
        Ok(gpu) => gpu,
        Err(err) => {
            assert!(
                std::env::var("X3_REQUIRE_GPU").as_deref() != Ok("1"),
                "X3_REQUIRE_GPU=1 but no GPU backend: {err}"
            );
            eprintln!("gpu_secp256k1_matches_cpu_on_every_job: SKIPPED ({err})");
            return;
        }
    };
    let cases = cases();
    let jobs: Vec<_> = cases.iter().map(|c| c.job.clone()).collect();
    let cpu = CpuBackend::new().verify_secp256k1_batch(&jobs).unwrap();
    let started = std::time::Instant::now();
    let got = gpu.verify_secp256k1_batch(&jobs).unwrap();
    eprintln!("{} jobs on GPU in {:?}", jobs.len(), started.elapsed());

    let mismatches: Vec<String> = cases
        .iter()
        .zip(cpu.iter().zip(&got))
        .filter(|(_, (c, g))| c != g)
        .map(|(case, (c, g))| format!("{}: cpu={c} gpu={g}", case.label))
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} mismatches, first: {:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(10)]
    );

    // Not vacuous: plenty of both verdicts, and the edge cases resolved as the
    // reference library resolves them.
    let valid = cpu.iter().filter(|v| **v).count();
    assert!(
        valid > 1500 && jobs.len() - valid > 1500,
        "valid={valid} of {}",
        jobs.len()
    );
    let verdict = |label: &str| {
        cases
            .iter()
            .zip(&cpu)
            .find(|(c, _)| c.label == label)
            .map(|(_, v)| *v)
            .unwrap()
    };
    assert!(
        verdict("r+n: x(R) >= n, r = x(R) - n"),
        "the reference must accept the r+n signature"
    );
    assert!(!verdict("r+n: unreduced r = x(R) >= n"));
    assert!(verdict("Q = G #0") && verdict("Q = -G #0"));
    assert!(verdict("Q = lambda G #0") && verdict("Q = -lambda G #0"));
    assert!(verdict("0 hybrid good parity") && !verdict("0 hybrid wrong parity"));
    assert!(!verdict("0 high-S"));

    // And through the fail-closed parity wrapper callers use.
    assert_eq!(secp256k1_with_parity(&gpu, &jobs).unwrap(), cpu);
}

#[test]
fn multi_gpu_split_matches_cpu_on_every_job() {
    let adapters = x3_accel_wgpu::WgpuBackend::hardware_adapters().len();
    if adapters < 2 {
        assert!(
            std::env::var("X3_REQUIRE_MULTI_GPU").as_deref() != Ok("1"),
            "X3_REQUIRE_MULTI_GPU=1 but only {adapters} adapter(s)"
        );
        eprintln!("multi_gpu_split_matches_cpu_on_every_job: SKIPPED ({adapters} adapter(s))");
        return;
    }
    // min_split 1: force every batch to be split, so each device sees a
    // slice of the edge cases and the merge order is exercised.
    let multi = x3_accel::MultiDevice::calibrated_wgpu(1).unwrap();
    assert_eq!(
        multi.device_count(),
        adapters,
        "a device failed calibration"
    );
    let cases = cases();
    let jobs: Vec<_> = cases.iter().map(|c| c.job.clone()).collect();
    let cpu = CpuBackend::new().verify_secp256k1_batch(&jobs).unwrap();
    let got = multi.verify_secp256k1_batch(&jobs).unwrap();
    let mismatches: Vec<_> = cases
        .iter()
        .zip(cpu.iter().zip(&got))
        .filter(|(_, (c, g))| c != g)
        .map(|(case, (c, g))| format!("{}: cpu={c} gpu={g}", case.label))
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:?}");
    eprintln!(
        "split over {} devices, weights {:?}",
        multi.device_count(),
        multi.weights()
    );
}

/// The kernel enforces low-S itself, so a host-side check that let a high-S
/// signature through could only cause a rejection. Fed straight to the kernel,
/// past `prepare_secp256k1`, the high-S twin of a valid signature is refused
/// while the signature itself is accepted.
#[test]
fn kernel_rejects_high_s_without_the_host_check() {
    let gpu = match x3_accel_wgpu::WgpuBackend::initialize() {
        Ok(gpu) => gpu,
        Err(err) => {
            assert!(
                std::env::var("X3_REQUIRE_GPU").as_deref() != Ok("1"),
                "X3_REQUIRE_GPU=1 but no GPU backend: {err}"
            );
            eprintln!("kernel_rejects_high_s_without_the_host_check: SKIPPED ({err})");
            return;
        }
    };
    let secp = Secp256k1::new();
    let sk = SecretKey::from_slice(&[0x42; 32]).unwrap();
    let z = [0x24u8; 32];
    let sig = secp
        .sign_ecdsa(&Message::from_digest(z), &sk)
        .serialize_compact();
    let point = PublicKey::from_secret_key(&secp, &sk).serialize_uncompressed();
    let mut r = [0u8; 32];
    r.copy_from_slice(&sig[..32]);
    let mut low_s = [0u8; 32];
    low_s.copy_from_slice(&sig[32..]);
    let high_s = be32(&(n() - BigUint::from_bytes_be(&low_s)));
    let mut qx = [0u8; 32];
    qx.copy_from_slice(&point[1..33]);
    let mut qy = [0u8; 32];
    qy.copy_from_slice(&point[33..65]);
    let job = |s| x3_accel_wgpu::Secp256k1Prepared { r, s, z, qx, qy };
    assert_eq!(
        gpu.secp256k1_verify_prepared(&[job(low_s), job(high_s)])
            .unwrap(),
        vec![true, false]
    );
}
