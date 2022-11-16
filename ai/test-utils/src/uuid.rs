// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Test utils for UUID.

use uuid::Uuid;

/// Creates an UUID by combining `fcb6a685-eb92-4d36-8686-XXXXXXXXXXXX` with the given `sub_id`.
pub const fn mock_uuid(sub_id: usize) -> Uuid {
    const BASE_UUID: u128 = 0xfcb6_a685_eb92_4d36_8686_0000_0000_0000;
    Uuid::from_u128(BASE_UUID | (sub_id as u128))
}

#[test]
fn test_mock_uuid() {
    assert_eq!(
        format!("{}", mock_uuid(0xABCD_EF0A)),
        "fcb6a685-eb92-4d36-8686-0000abcdef0a",
    );
}
