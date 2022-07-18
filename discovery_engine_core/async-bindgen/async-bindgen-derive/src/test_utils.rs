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

macro_rules! assert_rust_code_eq {
    ($left:expr, $right:expr) => {{
        let left = $left;
        let right = $right;

        let left_syn: syn::File = syn::parse_str(left.as_ref()).expect("parsing left failed");
        let right_syn: syn::File = syn::parse_str(right.as_ref()).expect("parsing right failed");

        if left_syn != right_syn {
            panic!("Code is not AST equal.\nLEFT: {}\nRIGHT: {}", left, right);
        }
    }};
}

pub(crate) use assert_rust_code_eq;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_equal_ignores_formatting() {
        assert_rust_code_eq!("fn a(x: u32) {}", "fn a(\n\tx : u32\n) {}");
    }
}
