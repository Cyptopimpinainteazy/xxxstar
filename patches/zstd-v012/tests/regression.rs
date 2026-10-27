use std::io::{Read, Write};

use zstd::dict::{DecoderDictionary, EncoderDictionary};

#[test]
fn prepared_dictionary_round_trip() {
    let dictionary = b"x3-mainnet-zstd-regression-dictionary";
    let payload = b"x3-mainnet-zstd-regression-payload";

    let encoder_dictionary = EncoderDictionary::copy(dictionary, 3);
    let mut encoder = zstd::stream::Encoder::with_prepared_dictionary(
        Vec::new(),
        &encoder_dictionary,
    )
    .expect("create encoder");
    encoder.write_all(payload).expect("write payload");
    let compressed = encoder.finish().expect("finish encoder");

    let decoder_dictionary = DecoderDictionary::copy(dictionary);
    let mut decoder = zstd::stream::Decoder::with_prepared_dictionary(
        &compressed[..],
        &decoder_dictionary,
    )
    .expect("create decoder");
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded).expect("decode payload");

    assert_eq!(decoded, payload);
}
