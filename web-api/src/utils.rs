// Copyright 2022 Xayn AG
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

use derive_more::Deref;
use figment::value::magic::RelativePathBuf as FigmentRelativePathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Deref)]
#[serde(transparent)]
pub struct RelativePathBuf {
    #[serde(serialize_with = "FigmentRelativePathBuf::serialize_relative")]
    inner: FigmentRelativePathBuf,
}

impl From<&str> for RelativePathBuf {
    fn from(s: &str) -> Self {
        Self {
            inner: FigmentRelativePathBuf::from(s),
        }
    }
}

/// Appends a deprecation header.
// https://datatracker.ietf.org/doc/html/draft-dalal-deprecation-header-00
macro_rules! deprecate {
    // Appends deprecation header.
    (@header $customize:expr) => {
        $customize.append_header((
            ::actix_web::http::header::HeaderName::from_static("deprecation"),
            ::actix_web::http::header::HeaderValue::from_static("version=\"current\""),
        ))
    };
    // Marks a route as deprecated.
    ($fn:ident($($args:tt)*)) => {
        |$($args)*| async {
            deprecate!(@header $fn($($args)*).await.customize())
        }
    };
    // Conditionally marks a response as deprecated.
    (if $is_deprecated:ident $response:block) => {{
        let mut response = $response.customize();
        if $is_deprecated {
            response = deprecate!(@header response);
        }
        response
    }}
}
pub(crate) use deprecate;
