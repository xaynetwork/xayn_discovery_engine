use uuid::Uuid;

/// Creates an UUID by combining `fcb6a685-eb92-4d36-8686-XXXXXXXXXXXX` with the given `sub_id`.
pub(crate) const fn mock_uuid(sub_id: usize) -> Uuid {
    const BASE_UUID: u128 = 0xfcb6a685eb924d368686000000000000;
    Uuid::from_u128(BASE_UUID | (sub_id as u128))
}

#[test]
fn test_mock_uuid() {
    assert_eq!(
        format!("{}", mock_uuid(0xABCDEF0A)),
        "fcb6a685-eb92-4d36-8686-0000abcdef0a",
    );
}
