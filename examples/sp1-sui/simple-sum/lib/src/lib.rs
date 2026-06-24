pub fn checked_sum(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

#[cfg(test)]
mod tests {
    use super::checked_sum;

    #[test]
    fn checked_sum_returns_sum_without_overflow() {
        assert_eq!(checked_sum(17, 25), Some(42));
    }

    #[test]
    fn checked_sum_rejects_u32_overflow() {
        assert_eq!(checked_sum(u32::MAX, 1), None);
    }
}
