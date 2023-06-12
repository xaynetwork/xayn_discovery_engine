// Copyright 2023 Xayn AG
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

use std::str::FromStr;

use reqwest::Url;

use crate::SetupError;

pub const NO_SEGMENTS: [&str; 0] = [];
pub const NO_PARAMS: [(&str, Option<&str>); 0] = [];
pub const NO_PARAM_VALUE: Option<&str> = None;
pub const NO_BODY: Option<()> = None;

#[derive(Clone, Debug)]
pub(super) struct SegmentableUrl(Url);

impl SegmentableUrl {
    pub(super) fn with_segments(
        mut self,
        segments: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        self.0.path_segments_mut()
            .unwrap(/* checked in constructor */)
            .extend(segments);

        self
    }

    pub(super) fn with_replaced_last_segment(mut self, last_segment: impl AsRef<str>) -> Self {
        self.0.path_segments_mut().unwrap(/* checked in constructor */).pop().push(last_segment.as_ref());

        self
    }

    pub(super) fn with_params(
        mut self,
        params: impl IntoIterator<Item = (impl AsRef<str>, Option<impl AsRef<str>>)>,
    ) -> Self {
        let mut query_pairs = self.0.query_pairs_mut();
        for (key, value) in params {
            if let Some(value) = value {
                query_pairs.append_pair(key.as_ref(), value.as_ref());
            } else {
                query_pairs.append_key_only(key.as_ref());
            }
        }
        drop(query_pairs);

        self
    }

    pub(super) fn into_inner(self) -> Url {
        self.0
    }
}

impl TryFrom<Url> for SegmentableUrl {
    type Error = SetupError;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        if url.cannot_be_a_base() {
            Err(anyhow::anyhow!("non segmentable url"))
        } else {
            Ok(Self(url))
        }
    }
}

impl FromStr for SegmentableUrl {
    type Err = SetupError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Url>()?.try_into()
    }
}
