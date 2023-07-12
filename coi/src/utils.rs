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

/// The number of seconds per day (without leap seconds).
pub(crate) const SECONDS_PER_DAY: u64 = 60 * 60 * 24;

/// Serde of a duration as full days (rounds down).
pub(crate) mod serde_duration_as_days {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::utils::SECONDS_PER_DAY;

    pub(crate) fn serialize<S>(horizon: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (horizon.as_secs() / SECONDS_PER_DAY).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(|days| Duration::from_secs(SECONDS_PER_DAY * days))
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, time::Duration};

    use serde::{Deserialize, Serialize};
    use serde_json::{from_str, to_string};

    use super::*;

    #[derive(Deserialize, Serialize)]
    struct Days(#[serde(with = "serde_duration_as_days")] Duration);

    #[test]
    fn test_less() -> Result<(), Box<dyn Error>> {
        let duration = Duration::from_secs(SECONDS_PER_DAY - 1);
        let serialized = to_string(&Days(duration))?;
        assert_eq!(serialized, "0");
        let deserialized = from_str::<Days>(&serialized)?.0;
        assert_eq!(deserialized, Duration::ZERO);
        Ok(())
    }

    #[test]
    fn test_equal() -> Result<(), Box<dyn Error>> {
        let duration = Duration::from_secs(SECONDS_PER_DAY);
        let serialized = to_string(&Days(duration))?;
        assert_eq!(serialized, "1");
        let deserialized = from_str::<Days>(&serialized)?.0;
        assert_eq!(deserialized, duration);
        Ok(())
    }

    #[test]
    fn test_greater() -> Result<(), Box<dyn Error>> {
        let duration = Duration::from_secs(SECONDS_PER_DAY + 1);
        let serialized = to_string(&Days(duration))?;
        assert_eq!(serialized, "1");
        let deserialized = from_str::<Days>(&serialized)?.0;
        assert_eq!(deserialized, Duration::from_secs(SECONDS_PER_DAY));
        Ok(())
    }
}
