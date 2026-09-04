#![deny(unsafe_code)]
#![allow(clippy::result_large_err)]

fn main() -> sc_cli::Result<()> {
    x3_chain_node::run()
}
