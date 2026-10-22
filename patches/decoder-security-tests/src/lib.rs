#[test]
fn reject_block_larger_than_declared_window() {
    // RFC8878: 1KiB window, final RLE block declaring 2KiB output.
    // Deliberately tiny expansion: reproduces the missing bound without OOM.
    let frame: &[u8] = &[0x28, 0xb5, 0x2f, 0xfd, 0, 0, 3, 0x40, 0, 0x61];
    let mut input = frame;
    let mut decoder = ruzstd::decoding::FrameDecoder::new();
    decoder.init(&mut input).unwrap();
    let result = decoder.decode_blocks(
        &mut input,
        ruzstd::decoding::BlockDecodingStrategy::UptoBytes(4096),
    );
    assert!(result.is_err(), "oversized block was accepted: {result:?}");
}

#[test]
fn valid_block_within_declared_window_still_decodes() {
    let frame: &[u8] = &[0x28, 0xb5, 0x2f, 0xfd, 0, 0, 3, 0x20, 0, 0x61];
    let mut input = frame;
    let mut decoder = ruzstd::decoding::FrameDecoder::new();
    decoder.init(&mut input).unwrap();
    assert!(decoder
        .decode_blocks(
            &mut input,
            ruzstd::decoding::BlockDecodingStrategy::UptoBytes(4096)
        )
        .unwrap());
    assert_eq!(decoder.collect().unwrap(), vec![0x61; 1024]);
}
