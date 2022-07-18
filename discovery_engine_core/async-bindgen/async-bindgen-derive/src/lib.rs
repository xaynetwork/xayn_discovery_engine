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

//! Derive crate for the `async_bindgen` crate.
#![deny(
    clippy::pedantic,
    clippy::future_not_send,
    clippy::missing_errors_doc,
    noop_method_call,
    rust_2018_idioms,
    rust_2021_compatibility,
    unused_qualifications,
    unsafe_op_in_unsafe_fn
)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::must_use_candidate, clippy::items_after_statements)]

use std::{
    env,
    fs::{self, create_dir_all},
    io,
    path::{Path, PathBuf},
};

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::Error;

use crate::{
    generate::{generate_extern_functions, generate_type, generate_type_import},
    parse::api::Api,
};

mod generate;
mod parse;
#[cfg(test)]
mod test_utils;
mod utils;

/// The `async_bindgen::api` proc macro.
#[proc_macro_attribute]
pub fn api(attrs: TokenStream1, item: TokenStream1) -> TokenStream1 {
    api2(attrs.into(), item.into())
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn api2(attrs: TokenStream2, item: TokenStream2) -> Result<TokenStream2, Error> {
    let res = parse_gen_api(attrs, item)?;
    let crate_root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    res.write_ext_file(&crate_root).map_err(|err| {
        Error::new(
            Span::call_site(),
            format!("Failed to write binding file: {}", err),
        )
    })?;
    Ok(res.into_token_stream())
}

fn parse_gen_api(attrs: TokenStream2, item: TokenStream2) -> Result<AsyncBindgenResult, Error> {
    let mut proc_tokens = item.clone();
    let api = Api::parse(attrs, item)?;

    let mut file_tokens = quote! {
        #![doc(hidden)]
        #![allow(
            clippy::unused_unit,
            clippy::semicolon_if_nothing_returned,
            clippy::used_underscore_binding,
            clippy::module_name_repetitions,
            unreachable_pub,
        )]
    };
    file_tokens.extend(api.header_code().clone());
    file_tokens.extend(generate_type(&api));
    proc_tokens.extend(api.header_code().clone());
    proc_tokens.extend(generate_type_import(&api));

    for lang in Language::languages() {
        file_tokens.extend(generate_extern_functions(&api, lang));
    }

    Ok(AsyncBindgenResult {
        api,
        proc_tokens,
        file_tokens,
    })
}

struct AsyncBindgenResult {
    api: Api,
    proc_tokens: TokenStream2,
    file_tokens: TokenStream2,
}

impl AsyncBindgenResult {
    pub(crate) fn into_token_stream(self) -> TokenStream2 {
        self.proc_tokens
    }

    pub(crate) fn write_ext_file(&self, crate_root: &Path) -> Result<(), io::Error> {
        let (file, content) = self.create_ext_file_output_info(crate_root);
        create_dir_all(file.parent().unwrap())?;
        fs::write(file, content)?;
        Ok(())
    }

    pub(crate) fn create_ext_file_output_info(&self, crate_root: &Path) -> (PathBuf, String) {
        let file = crate_root
            .join("src")
            .join("async_bindings")
            .join(self.api.mod_name().to_string())
            .with_extension("rs");
        let content = self.file_tokens.to_string();
        (file, content)
    }
}

#[derive(Clone, Copy)]
enum Language {
    Dart,
}

impl Language {
    pub(crate) fn languages() -> impl Iterator<Item = Self> {
        IntoIterator::into_iter([Language::Dart])
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_utils::assert_rust_code_eq;

    fn test_codegen(func: &str, expected_expanded_code: &str, expected_file_code: &str) {
        let attr = TokenStream2::new();
        let item = syn::parse_str::<TokenStream2>(func).unwrap();

        let result = parse_gen_api(attr, item).unwrap();
        let (_path, content) = result.create_ext_file_output_info(Path::new("/foo/"));
        let expansion = result.into_token_stream();
        assert_rust_code_eq!(expansion.to_string(), expected_expanded_code);
        assert_rust_code_eq!(content, expected_file_code);
    }

    #[test]
    fn test_codegen_no_arguments() {
        test_codegen(
            r#"
            impl BarFoot {
                pub async fn dodo() -> *const u8 { todo!() }
            }
        "#,
            r#"
            impl BarFoot {
                pub async fn dodo() -> *const u8 { todo!() }
            }

            pub use crate::async_bindings::bar_foot::BarFoot;
        "#,
            r##"
            #![doc(hidden)]
            #![allow(
                clippy::unused_unit,
                clippy::semicolon_if_nothing_returned,
                clippy::used_underscore_binding,
                clippy::module_name_repetitions,
                unreachable_pub,
            )]

            pub struct BarFoot;
            #[doc = r" Initializes the dart api."]
            #[doc = r""]
            #[doc = r" It's safe to be called multiple times and from multiple threads."]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" Must be called with a pointer produced by dart using"]
            #[doc = r" `NativeApi.initializeApiDLData`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_init_api__bar_foot(
                init_data: *mut ::std::ffi::c_void
            ) -> u8 {
                unsafe { ::async_bindgen::dart::initialize_dart_api_dl(init_data).is_ok().into() }
            }
            #[doc = r" Wrapper for initiating the call to an async function."]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
            #[doc = r" the rust-future completed or was dropped."]
            #[doc = r""]
            #[doc = r" This means non-owning references to extern allocated memory you pass in must"]
            #[doc = r" not be:"]
            #[doc = r""]
            #[doc = r" - deallocated"]
            #[doc = r" - accessed (if `&mut`)"]
            #[doc = r" - modified (if `&`)"]
            #[doc = r""]
            #[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_call__bar_foot__dodo(
                async_bindgen_dart_port_id: i64,
                async_bindgen_dart_completer_id: i64
            ) -> u8 {
                match ::async_bindgen::dart::PreparedCompleter::new(
                    async_bindgen_dart_port_id,
                    async_bindgen_dart_completer_id
                ) {
                    Ok(completer) => {
                        unsafe { completer.spawn(BarFoot::dodo()) };
                        1
                    }
                    Err(_) => 0
                }
            }
            #[doc = r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" See the language specific version of `PreparedCompleter::extract_result()`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_return__bar_foot__dodo(handle: i64) -> *const u8 {
                unsafe { ::async_bindgen::dart::PreparedCompleter::extract_result(handle) }
            }
        "##,
        );
    }

    #[test]
    fn test_codegen_multiple_arguments() {
        test_codegen(
            r#"
            impl BarFoot {
                pub async fn dork(x: i32, y: *const i32) -> isize { todo!() }
            }
        "#,
            r#"
            impl BarFoot {
                pub async fn dork(x: i32, y: *const i32) -> isize { todo!() }
            }

            pub use crate::async_bindings::bar_foot::BarFoot;
        "#,
            r##"
            #![doc(hidden)]
            #![allow(
                clippy::unused_unit,
                clippy::semicolon_if_nothing_returned,
                clippy::used_underscore_binding,
                clippy::module_name_repetitions,
                unreachable_pub,
            )]
                        pub struct BarFoot;
            #[doc = r" Initializes the dart api."]
            #[doc = r""]
            #[doc = r" It's safe to be called multiple times and from multiple threads."]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" Must be called with a pointer produced by dart using"]
            #[doc = r" `NativeApi.initializeApiDLData`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_init_api__bar_foot(
                init_data: *mut ::std::ffi::c_void
            ) -> u8 {
                unsafe { ::async_bindgen::dart::initialize_dart_api_dl(init_data).is_ok().into() }
            }
            #[doc = r" Wrapper for initiating the call to an async function."]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
            #[doc = r" the rust-future completed or was dropped."]
            #[doc = r""]
            #[doc = r" This means non-owning references to extern allocated memory you pass in must"]
            #[doc = r" not be:"]
            #[doc = r""]
            #[doc = r" - deallocated"]
            #[doc = r" - accessed (if `&mut`)"]
            #[doc = r" - modified (if `&`)"]
            #[doc = r""]
            #[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_call__bar_foot__dork(
                x: i32,
                y: *const i32,
                async_bindgen_dart_port_id: i64,
                async_bindgen_dart_completer_id: i64
            ) -> u8 {
                match ::async_bindgen::dart::PreparedCompleter::new(
                    async_bindgen_dart_port_id,
                    async_bindgen_dart_completer_id
                ) {
                    Ok(completer) => {
                        unsafe { completer.spawn(BarFoot::dork(x, y)) };
                        1
                    }
                    Err(_) => 0
                }
            }
            #[doc = r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" See the language specific version of `PreparedCompleter::extract_result()`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_return__bar_foot__dork(handle: i64) -> isize {
                unsafe { ::async_bindgen::dart::PreparedCompleter::extract_result(handle) }
            }
        "##,
        );
    }

    #[test]
    fn test_codegen_multiple_functions() {
        #![allow(clippy::too_many_lines)]

        test_codegen(
            r#"
            impl BarFoot {
                pub async fn dodo() -> *const u8 { todo!() }
                pub async fn dork(x: i32, y: *const i32) -> isize { todo!() }
            }
        "#,
            r#"
            impl BarFoot {
                pub async fn dodo() -> *const u8 { todo!() }
                pub async fn dork(x: i32, y: *const i32) -> isize { todo!() }
            }

            pub use crate::async_bindings::bar_foot::BarFoot;
        "#,
            r##"
            #![doc(hidden)]
            #![allow(
                clippy::unused_unit,
                clippy::semicolon_if_nothing_returned,
                clippy::used_underscore_binding,
                clippy::module_name_repetitions,
                unreachable_pub,
            )]
                        pub struct BarFoot;
            #[doc = r" Initializes the dart api."]
            #[doc = r""]
            #[doc = r" It's safe to be called multiple times and from multiple threads."]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" Must be called with a pointer produced by dart using"]
            #[doc = r" `NativeApi.initializeApiDLData`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_init_api__bar_foot(
                init_data: *mut ::std::ffi::c_void
            ) -> u8 {
                unsafe { ::async_bindgen::dart::initialize_dart_api_dl(init_data).is_ok().into() }
            }
            #[doc = r" Wrapper for initiating the call to an async function."]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
            #[doc = r" the rust-future completed or was dropped."]
            #[doc = r""]
            #[doc = r" This means non-owning references to extern allocated memory you pass in must"]
            #[doc = r" not be:"]
            #[doc = r""]
            #[doc = r" - deallocated"]
            #[doc = r" - accessed (if `&mut`)"]
            #[doc = r" - modified (if `&`)"]
            #[doc = r""]
            #[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_call__bar_foot__dodo(
                async_bindgen_dart_port_id: i64,
                async_bindgen_dart_completer_id: i64
            ) -> u8 {
                match ::async_bindgen::dart::PreparedCompleter::new(
                    async_bindgen_dart_port_id,
                    async_bindgen_dart_completer_id
                ) {
                    Ok(completer) => {
                        unsafe { completer.spawn(BarFoot::dodo()) };
                        1
                    }
                    Err(_) => 0
                }
            }
            #[doc = r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" See the language specific version of `PreparedCompleter::extract_result()`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_return__bar_foot__dodo(handle: i64) -> *const u8 {
                unsafe { ::async_bindgen::dart::PreparedCompleter::extract_result(handle) }
            }
            #[doc = r" Wrapper for initiating the call to an async function."]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
            #[doc = r" the rust-future completed or was dropped."]
            #[doc = r""]
            #[doc = r" This means non-owning references to extern allocated memory you pass in must"]
            #[doc = r" not be:"]
            #[doc = r""]
            #[doc = r" - deallocated"]
            #[doc = r" - accessed (if `&mut`)"]
            #[doc = r" - modified (if `&`)"]
            #[doc = r""]
            #[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_call__bar_foot__dork(
                x: i32,
                y: *const i32,
                async_bindgen_dart_port_id: i64,
                async_bindgen_dart_completer_id: i64
            ) -> u8 {
                match ::async_bindgen::dart::PreparedCompleter::new(
                    async_bindgen_dart_port_id,
                    async_bindgen_dart_completer_id
                ) {
                    Ok(completer) => {
                        unsafe { completer.spawn(BarFoot::dork(x, y)) };
                        1
                    }
                    Err(_) => 0
                }
            }
            #[doc = r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
            #[doc = r""]
            #[doc = r" # Safety"]
            #[doc = r""]
            #[doc = r" See the language specific version of `PreparedCompleter::extract_result()`."]
            #[no_mangle]
            pub unsafe extern "C" fn async_bindgen_dart_return__bar_foot__dork(handle: i64) -> isize {
                unsafe { ::async_bindgen::dart::PreparedCompleter::extract_result(handle) }
            }
        "##,
        );
    }
}
