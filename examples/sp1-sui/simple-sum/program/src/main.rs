#![no_main]

sp1_zkvm::entrypoint!(main);

use simple_sum_lib::checked_sum;

pub fn main() {
    let a = sp1_zkvm::io::read::<u32>();
    let b = sp1_zkvm::io::read::<u32>();
    let sum = checked_sum(a, b).expect("u32 addition overflow");

    sp1_zkvm::io::commit(&a);
    sp1_zkvm::io::commit(&b);
    sp1_zkvm::io::commit(&sum);
}
