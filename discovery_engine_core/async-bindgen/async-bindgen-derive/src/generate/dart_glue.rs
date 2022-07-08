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

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Path};

use crate::{parse::function::FunctionInput, utils::type_from_name};

pub(crate) fn additional_dart_inputs() -> Vec<FunctionInput> {
    vec![
        FunctionInput::new(
            Ident::new("async_bindgen_dart_port_id", Span::call_site()),
            type_from_name("i64"),
        ),
        FunctionInput::new(
            Ident::new("async_bindgen_dart_completer_id", Span::call_site()),
            type_from_name("i64"),
        ),
    ]
}

pub(crate) fn path_prefix() -> Path {
    syn::parse_str("::async_bindgen::dart").unwrap()
}

pub(crate) fn call_name(api_name: &Ident, fn_name: &Ident) -> Ident {
    Ident::new(
        &format!("async_bindgen_dart_call__{}__{}", api_name, fn_name),
        fn_name.span(),
    )
}

pub(crate) fn ret_name(api_name: &Ident, fn_name: &Ident) -> Ident {
    Ident::new(
        &format!("async_bindgen_dart_return__{}__{}", api_name, fn_name),
        fn_name.span(),
    )
}

pub(crate) fn generate_dart_api_init(api_name: &Ident) -> TokenStream {
    let init_name = Ident::new(
        &format!("async_bindgen_dart_init_api__{}", api_name),
        api_name.span(),
    );

    quote! {
        /// Initializes the dart api.
        ///
        /// It's safe to be called multiple times and from multiple threads.
        ///
        /// # Safety
        ///
        /// Must be called with a pointer produced by dart using
        /// `NativeApi.initializeApiDLData`.
        #[no_mangle]
        pub unsafe extern "C" fn #init_name(init_data: *mut ::std::ffi::c_void) -> u8 {
            unsafe { ::async_bindgen::dart::initialize_dart_api_dl(init_data).is_ok().into() }
        }
    }
}
