use std::process;

use x3_chain_node::service;

fn main() {
	println!("X3 Chain Node v{}", env!("CARGO_PKG_VERSION"));
	println!("Dual-VM execution (EVM + SVM) with native assets & atomic cross-chain operations.");
	process::exit(0);
}
