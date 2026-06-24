use alloy_sol_types::sol;

sol! {
    /// The public values encoded as a struct that can be decoded by downstream verifiers.
    struct PublicValuesStruct {
        uint32 n;
        uint32 a;
        uint32 b;
    }
}

/// Compute the nth Fibonacci step using wrapping u32 arithmetic, matching the upstream example.
pub fn fibonacci(n: u32) -> (u32, u32) {
    let mut a = 0u32;
    let mut b = 1u32;
    for _ in 0..n {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
    }
    (a, b)
}

#[cfg(test)]
mod tests {
    use super::fibonacci;

    #[test]
    fn computes_small_fibonacci_pair() {
        assert_eq!(fibonacci(20), (6765, 10946));
    }
}
