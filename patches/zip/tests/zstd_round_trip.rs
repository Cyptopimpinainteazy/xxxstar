use std::io::{Read, Write};

use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[test]
fn zstd_compressed_archive_round_trip() {
    let mut archive_bytes = Vec::new();
    {
        let mut writer = ZipWriter::new(std::io::Cursor::new(&mut archive_bytes));
        let options = FileOptions::default().compression_method(CompressionMethod::Zstd);
        writer.start_file("payload.txt", options).expect("start file");
        writer
            .write_all(b"x3-mainnet-zip-zstd-regression")
            .expect("write file");
        writer.finish().expect("finish archive");
    }

    let mut archive =
        ZipArchive::new(std::io::Cursor::new(archive_bytes)).expect("open archive");
    let mut payload = String::new();
    archive
        .by_name("payload.txt")
        .expect("find file")
        .read_to_string(&mut payload)
        .expect("read file");

    assert_eq!(payload, "x3-mainnet-zip-zstd-regression");
}
