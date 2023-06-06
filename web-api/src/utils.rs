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

macro_rules! json_object {
    ({ $($tt:tt)* }) => ({
        let ::serde_json::Value::Object(obj) = json!({ $($tt)* }) else {
            unreachable!(/* the {} enforces it's always an object */);
        };
        obj
    });
}

pub(crate) use json_object;
