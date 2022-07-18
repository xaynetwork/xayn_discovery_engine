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

use proc_macro2::Span;
use syn::{punctuated::Punctuated, Ident, Path, PathArguments, PathSegment, Type, TypePath};

/// Using a path prefix and a string name to create a type.
///
/// The name will be turned into an `Ident` and then
/// added to the `path_prefix` to create a [`Type::Path`]
/// variant.
pub(crate) fn type_from_path_and_name(path_prefix: Path, name: &str) -> Type {
    let mut path = path_prefix;
    path.segments.push(PathSegment {
        ident: Ident::new(name, Span::call_site()),
        arguments: PathArguments::None,
    });

    Type::Path(TypePath { qself: None, path })
}

pub(crate) fn type_from_name(name: &str) -> Type {
    let mut segments = Punctuated::new();
    segments.push(PathSegment {
        ident: Ident::new(name, Span::call_site()),
        arguments: PathArguments::None,
    });

    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    })
}
