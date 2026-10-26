use std::io::{Read, Write};
use zstd::stream::{read::Decoder, write::Encoder};

#[test]
fn encoder_with_context_starts_a_new_frame() {
    let mut cctx = zstd::zstd_safe::CCtx::create();
    Encoder::with_context(Vec::new(), &mut cctx)
        .write_all(b"first job")
        .unwrap();

    let mut encoder = Encoder::with_context(Vec::new(), &mut cctx);
    encoder.write_all(b"second job").unwrap();
    let second = encoder.finish().unwrap();
    assert_eq!(zstd::decode_all(&second[..]).unwrap(), b"second job");
}

#[test]
fn decoder_with_context_starts_a_new_frame() {
    let first = zstd::encode_all(&b"first job"[..], 1).unwrap();
    let second = zstd::encode_all(&b"second job"[..], 1).unwrap();

    let mut dctx = zstd::zstd_safe::DCtx::create();
    Decoder::with_context(&first[..], &mut dctx)
        .read_exact(&mut [0u8; 5])
        .unwrap();

    let mut output = Vec::new();
    Decoder::with_context(&second[..], &mut dctx)
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, b"second job");
}
