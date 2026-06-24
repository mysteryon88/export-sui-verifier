#![no_main]

sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use fibonacci_lib::{fibonacci, PublicValuesStruct};

pub fn main() {
    let n = sp1_zkvm::io::read::<u32>();
    let (a, b) = fibonacci(n);
    let bytes = PublicValuesStruct::abi_encode(&PublicValuesStruct { n, a, b });

    sp1_zkvm::io::commit_slice(&bytes);
}
