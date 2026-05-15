pub const VILLAGE_STREAM_TYPE: &str = "village";

#[must_use]
pub fn village_stream_id(village_id: u32) -> String {
    village_id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn village_stream_type_is_stable() {
        assert_eq!(VILLAGE_STREAM_TYPE, "village");
    }

    #[test]
    fn village_stream_id_uses_u32_plain_string() {
        assert_eq!(village_stream_id(0), "0");
        assert_eq!(village_stream_id(42), "42");
        assert_eq!(village_stream_id(u32::MAX), "4294967295");
    }
}
