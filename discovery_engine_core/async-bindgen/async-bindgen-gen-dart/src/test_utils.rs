// Copyright 2022 Xayn AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

macro_rules! assert_trimmed_line_eq {
    ($left:expr, $right:expr) => {{
        let left = $left;
        let mut left = $crate::test_utils::trimmed_non_empty_lines(&left);
        let right = $right;
        let mut right = $crate::test_utils::trimmed_non_empty_lines(&right);
        for (left, right) in (&mut left).zip(&mut right) {
            assert_eq!(left, right);
        }
        assert!(left.next().is_none());
        assert!(right.next().is_none());
    }};
}

pub(crate) use assert_trimmed_line_eq;

pub(crate) fn trimmed_non_empty_lines(s: &str) -> impl Iterator<Item = &str> {
    s.lines().flat_map(|line| {
        let line = line.trim();
        (!line.is_empty()).then(|| line)
    })
}
