use std::fs;
use std::path::PathBuf;

use x3_genesis_builder::GenesisManifest;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(input_path) = args.next() else {
        eprintln!("usage: x3-genesis-builder <manifest.json>");
        std::process::exit(2);
    };

    let input = PathBuf::from(input_path);
    let raw = match fs::read_to_string(&input) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("failed to read manifest file: {err}");
            std::process::exit(1);
        }
    };

    let manifest: GenesisManifest = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("failed to parse manifest json: {err}");
            std::process::exit(1);
        }
    };

    match manifest.digest_hex() {
        Ok(digest) => {
            println!("{digest}");
        }
        Err(err) => {
            eprintln!("invalid manifest: {err}");
            std::process::exit(1);
        }
    }
}
