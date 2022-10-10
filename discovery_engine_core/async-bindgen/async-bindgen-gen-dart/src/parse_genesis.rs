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

//! Ad-hoc parses a genesis.dart file for the type definitions produced by ffigen.
use std::collections::HashMap;

use heck::ToLowerCamelCase;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct AsyncFunctionSignature {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) ffi_call_name: String,
    pub(crate) ffi_return_name: String,
    pub(crate) output: String,
    pub(crate) inputs: Vec<DartFunctionInputs>,
}

impl AsyncFunctionSignature {
    fn from_call_and_ret_func(
        name: String,
        call_fn: DartFunctionSignature,
        ret_fn: DartFunctionSignature,
    ) -> Self {
        //FIXME better errors
        assert_eq!(call_fn.output, "int");
        assert_eq!(ret_fn.inputs.len(), 1);
        assert_eq!(ret_fn.inputs[0].r#type, "int");

        AsyncFunctionSignature {
            doc: call_fn.doc,
            name: name.to_lower_camel_case(),
            ffi_call_name: call_fn.name,
            ffi_return_name: ret_fn.name,
            output: ret_fn.output,
            inputs: call_fn.inputs,
        }
    }

    pub(crate) fn sniff_dart_signatures(
        dart_src: &str,
    ) -> HashMap<String, Vec<AsyncFunctionSignature>> {
        // mod_name => fn_name => (call_fn, ret_fn)
        let mut modules_to_functions_to_parts =
            HashMap::<String, HashMap<String, (Option<_>, Option<_>)>>::new();
        for captures in SNIFF_FUNCTION_REGEX.captures_iter(dart_src) {
            let func = DartFunctionSignature::from_captures(&captures);
            if let Some((mod_name, fn_name, is_call)) = split_dart_bindgen_name(&func.name) {
                let (call_slot, ret_slot) = modules_to_functions_to_parts
                    .entry(mod_name.into())
                    .or_default()
                    .entry(fn_name.into())
                    .or_default();

                let (slot, func) = if is_call {
                    (call_slot, func.without_extra_args())
                } else {
                    (ret_slot, func)
                };
                //FIXME proper error message
                assert!(slot.is_none());
                *slot = Some(func);
            }
        }

        modules_to_functions_to_parts
            .into_iter()
            .map(|(mod_name, functions)| {
                let functions = functions
                    .into_iter()
                    .map(|(name, (call_fn, ret_fn))| {
                        let call_fn = call_fn.unwrap_or_else(|| {
                            panic!(
                                "Part of async glue missing for {:?} in {:?}: call function.",
                                name, mod_name
                            )
                        });
                        let ret_fn = ret_fn.unwrap_or_else(|| {
                            panic!(
                                "Part of async glue missing for {:?} in {:?}: return function.",
                                name, mod_name
                            )
                        });
                        AsyncFunctionSignature::from_call_and_ret_func(name, call_fn, ret_fn)
                    })
                    .collect();
                (mod_name, functions)
            })
            .collect()
    }
}

/// Turns strings matching `async_bindgen_dart_(c|r)__<mod_name>__<fn_name>` into `(mod_name, fn_name, is_call)`.
fn split_dart_bindgen_name(name: &str) -> Option<(&str, &str, bool)> {
    let name = name.strip_prefix("async_bindgen_dart_")?;
    let is_call = name.starts_with('c');
    let name = name
        .strip_prefix("call__")
        .or_else(|| name.strip_prefix("return__"))?;
    let (mod_name, fn_name) = name.split_once("__")?;
    Some((mod_name, fn_name, is_call))
}

#[derive(Serialize)]
pub(crate) struct DartFunctionInputs {
    pub(crate) name: String,
    pub(crate) r#type: String,
}

pub(crate) struct DartFunctionSignature {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) output: String,
    pub(crate) inputs: Vec<DartFunctionInputs>,
}

impl DartFunctionSignature {
    fn from_captures(captures: &Captures) -> Self {
        //UNWRAP_SAFE: capture group is not optional
        let name = captures.name("func_name").unwrap().as_str().trim().into();
        let doc = get_doc_from_captures(captures);
        //UNWRAP_SAFE: capture group is not optional
        let output = captures.name("output").unwrap().as_str().trim().to_owned();
        let inputs = get_inputs_from_captures(captures);
        DartFunctionSignature {
            doc,
            name,
            output,
            inputs,
        }
    }

    fn without_extra_args(self) -> Self {
        let Self {
            doc,
            name,
            output,
            inputs,
        } = self;

        let inputs = inputs
            .into_iter()
            .filter(|input| !input.name.starts_with("async_bindgen_"))
            .collect();

        Self {
            doc,
            name,
            output,
            inputs,
        }
    }
}

fn get_doc_from_captures(captures: &Captures) -> Vec<String> {
    captures_as_trimmed_lines(captures, "doc")
        .map(ToOwned::to_owned)
        .collect()
}

fn get_inputs_from_captures(captures: &Captures) -> Vec<DartFunctionInputs> {
    captures_as_trimmed_lines(captures, "inputs")
        .flat_map(|line| {
            let line = line.trim_end_matches(',');
            let (r#type, name) = line.rsplit_once(' ')?;
            Some(DartFunctionInputs {
                name: name.to_owned(),
                r#type: r#type.to_owned(),
            })
        })
        .collect()
}

fn captures_as_trimmed_lines<'a>(
    captures: &'a Captures,
    name: &'_ str,
) -> impl Iterator<Item = &'a str> {
    captures
        .name(name)
        .map(|cap| cap.as_str())
        .unwrap_or("")
        .lines()
        .flat_map(|line| {
            let line = line.trim();
            (!line.is_empty()).then_some(line)
        })
}

static SNIFF_FUNCTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)
        (?:\n|^)
        (?P<doc>(?:\s*///(?:\s.*)?\n)*)
        \s*(?P<output>[a-zA-Z0-9_<>.]+)\s*(?P<func_name>[[:word:]]+)\(\n
            (?P<inputs>(?:\s+[a-zA-Z0-9_<>.]+\s[[:word:]]+,\n)*)
        \s*\)\s\{\n
    ",
    )
    .unwrap()
});

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use regex::Captures;

    use super::*;

    static TEST_DART_SRC: &str = r#"
    // AUTO GENERATED FILE, DO NOT EDIT.
    //
    // Generated by `package:ffigen`.
    import 'dart:ffi' as ffi;

    /// Bindings for the integration tests
    class IntegrationTestsFfi {
      /// Holds the symbol lookup function.
      final ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
          _lookup;

      /// The symbols are looked up in [dynamicLibrary].
      IntegrationTestsFfi(ffi.DynamicLibrary dynamicLibrary)
          : _lookup = dynamicLibrary.lookup;

      /// The symbols are looked up with [lookup].
      IntegrationTestsFfi.fromLookup(
          ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
              lookup)
          : _lookup = lookup;

      /// Initializes the dart api.
      ///
      /// Is safe to be called multiple times and form multiple
      /// thread.
      ///
      /// # Safety
      ///
      /// Must be called with a pointer produced by dart using
      /// `NativeApi.initializeApiDLData`.
      int async_bindgen_dart_init_api__async_api(
        ffi.Pointer<ffi.Void> init_data,
      ) {
        return _async_bindgen_dart_init_api__async_api(
          init_data,
        );
      }

      late final _async_bindgen_dart_init_api__async_apiPtr =
          _lookup<ffi.NativeFunction<ffi.Uint8 Function(ffi.Pointer<ffi.Void>)>>(
              'async_bindgen_dart_init_api__async_api');
      late final _async_bindgen_dart_init_api__async_api =
          _async_bindgen_dart_init_api__async_apiPtr
              .asFunction<int Function(ffi.Pointer<ffi.Void>)>();

      /// Wrapper for initiating the call to an async function.
      int async_bindgen_dart_call__async_api__foo_bar(
        int x,
        double y,
        int async_bindgen_dart_port_id,
        int async_bindgen_dart_completer_id,
      ) {
        return _async_bindgen_dart_call__async_api__foo_bar(
          x,
          y,
          async_bindgen_dart_port_id,
          async_bindgen_dart_completer_id,
        );
      }

      late final _async_bindgen_dart_call__async_api__addPtr = _lookup<
          ffi.NativeFunction<
              ffi.Uint8 Function(ffi.Uint8, ffi.Double, ffi.Int32,
                  ffi.Int64)>>('async_bindgen_dart_call__async_api__foo_bar');
      late final _async_bindgen_dart_call__async_api__add =
          _async_bindgen_dart_call__async_api__foo_barPtr
              .asFunction<int Function(int, double, int, int)>();

      /// Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`.
      ///
      /// # Safety
      ///
      /// See the language specific version of `PreparedCompleter::extract_result()`.
      ffi.Pointer<MyType> async_bindgen_dart_return__async_api__foo_bar(
        int handle,
      ) {
        return _async_bindgen_dart_return__async_api__foo_bar(
          handle,
        );
      }

      late final _async_bindgen_dart_return__async_api__foo_barPtr =
          _lookup<ffi.NativeFunction<ffi.Pointer<MyType> Function(ffi.Int64)>>(
              'async_bindgen_dart_return__async_api__foo_bar');
      late final _async_bindgen_dart_return__async_api__foo_bar =
          _async_bindgen_dart_return__async_api__foo_barPtr
              .asFunction<ffi.Pointer<MyType> Function(int)>();

      /// Wrapper for initiating the call to an async function.
      int async_bindgen_dart_call__async_api__sub(
        int x,
        int y,
        int async_bindgen_dart_port_id,
        int async_bindgen_dart_completer_id,
      ) {
        return _async_bindgen_dart_call__async_api__sub(
          x,
          y,
          async_bindgen_dart_port_id,
          async_bindgen_dart_completer_id,
        );
      }

      late final _async_bindgen_dart_call__async_api__subPtr = _lookup<
          ffi.NativeFunction<
              ffi.Uint8 Function(ffi.Uint8, ffi.Uint8, ffi.Int32,
                  ffi.Int64)>>('async_bindgen_dart_call__async_api__sub');
      late final _async_bindgen_dart_call__async_api__sub =
          _async_bindgen_dart_call__async_api__subPtr
              .asFunction<int Function(int, int, int, int)>();

      /// Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`.
      ///
      /// # Safety
      ///
      /// See the language specific version of `PreparedCompleter::extract_result()`.
      int async_bindgen_dart_return__async_api__sub(
        int handle,
      ) {
        return _async_bindgen_dart_return__async_api__sub(
          handle,
        );
      }

      late final _async_bindgen_dart_return__async_api__subPtr =
          _lookup<ffi.NativeFunction<ffi.Uint8 Function(ffi.Int64)>>(
              'async_bindgen_dart_return__async_api__sub');
      late final _async_bindgen_dart_return__async_api__sub =
          _async_bindgen_dart_return__async_api__subPtr
              .asFunction<int Function(int)>();

      /// Initializes the dart api.
      ///
      /// Is safe to be called multiple times and form multiple
      /// thread.
      ///
      /// # Safety
      ///
      /// Must be called with a pointer produced by dart using
      /// `NativeApi.initializeApiDLData`.
      int async_bindgen_dart_init_api__api2(
        ffi.Pointer<ffi.Void> init_data,
      ) {
        return _async_bindgen_dart_init_api__api2(
          init_data,
        );
      }

      late final _async_bindgen_dart_init_api__api2Ptr =
          _lookup<ffi.NativeFunction<ffi.Uint8 Function(ffi.Pointer<ffi.Void>)>>(
              'async_bindgen_dart_init_api__api2');
      late final _async_bindgen_dart_init_api__api2 =
          _async_bindgen_dart_init_api__api2Ptr
              .asFunction<int Function(ffi.Pointer<ffi.Void>)>();

      /// Wrapper for initiating the call to an async function.
      int async_bindgen_dart_call__api2__get_the_byte(
        int async_bindgen_dart_port_id,
        int async_bindgen_dart_completer_id,
      ) {
        return _async_bindgen_dart_call__api2__get_the_byte(
          async_bindgen_dart_port_id,
          async_bindgen_dart_completer_id,
        );
      }

      late final _async_bindgen_dart_call__api2__get_the_bytePtr =
          _lookup<ffi.NativeFunction<ffi.Uint8 Function(ffi.Int32, ffi.Int64)>>(
              'async_bindgen_dart_call__api2__get_the_byte');
      late final _async_bindgen_dart_call__api2__get_the_byte =
          _async_bindgen_dart_call__api2__get_the_bytePtr
              .asFunction<int Function(int, int)>();

      /// Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`.
      ///
      /// # Safety
      ///
      /// See the language specific version of `PreparedCompleter::extract_result()`.
      int async_bindgen_dart_return__api2__get_the_byte(
        int handle,
      ) {
        return _async_bindgen_dart_return__api2__get_the_byte(
          handle,
        );
      }

      late final _async_bindgen_dart_return__api2__get_the_bytePtr =
          _lookup<ffi.NativeFunction<ffi.Uint8 Function(ffi.Int64)>>(
              'async_bindgen_dart_return__api2__get_the_byte');
      late final _async_bindgen_dart_return__api2__get_the_byte =
          _async_bindgen_dart_return__api2__get_the_bytePtr
              .asFunction<int Function(int)>();
    }

    "#;

    #[test]
    fn test_sniffing() {
        let mut modules = AsyncFunctionSignature::sniff_dart_signatures(TEST_DART_SRC)
            .into_iter()
            .sorted_by(|l, r| l.0.cmp(&r.0));

        let (name, sigs) = modules.next().unwrap();
        assert_eq!(name, "api2");
        let mut sigs = sigs.into_iter().sorted_by(|l, r| l.name.cmp(&r.name));
        let sig = sigs.next().unwrap();
        assert_eq!(sig.name, "getTheByte");
        assert_eq!(
            sig.ffi_call_name,
            "async_bindgen_dart_call__api2__get_the_byte"
        );
        assert_eq!(
            sig.ffi_return_name,
            "async_bindgen_dart_return__api2__get_the_byte"
        );
        assert_eq!(sig.output, "int");
        assert!(sig.inputs.is_empty());
        assert!(sigs.next().is_none());

        let (name, sigs) = modules.next().unwrap();
        assert_eq!(name, "async_api");
        let mut sigs = sigs.into_iter().sorted_by(|l, r| l.name.cmp(&r.name));
        let sig = sigs.next().unwrap();
        assert_eq!(sig.name, "fooBar");
        assert_eq!(
            sig.ffi_call_name,
            "async_bindgen_dart_call__async_api__foo_bar"
        );
        assert_eq!(
            sig.ffi_return_name,
            "async_bindgen_dart_return__async_api__foo_bar"
        );
        assert_eq!(sig.output, "ffi.Pointer<MyType>");
        let mut inputs = sig.inputs.into_iter();
        let input = inputs.next().unwrap();
        assert_eq!(input.name, "x");
        assert_eq!(input.r#type, "int");
        let input = inputs.next().unwrap();
        assert_eq!(input.name, "y");
        assert_eq!(input.r#type, "double");
        assert!(inputs.next().is_none());

        let sig = sigs.next().unwrap();
        assert_eq!(sig.name, "sub");
        assert_eq!(sig.ffi_call_name, "async_bindgen_dart_call__async_api__sub");
        assert_eq!(
            sig.ffi_return_name,
            "async_bindgen_dart_return__async_api__sub"
        );
        assert_eq!(sig.output, "int");
        let mut inputs = sig.inputs.into_iter();
        let input = inputs.next().unwrap();
        assert_eq!(input.name, "x");
        assert_eq!(input.r#type, "int");
        let input = inputs.next().unwrap();
        assert_eq!(input.name, "y");
        assert_eq!(input.r#type, "int");
        assert!(inputs.next().is_none());
        assert!(sigs.next().is_none());

        assert!(modules.next().is_none());
    }

    #[test]
    fn test_regex_matches_function_sig() {
        let mut captures = SNIFF_FUNCTION_REGEX.captures_iter(TEST_DART_SRC);

        test_match(
            captures.next().unwrap(),
            vec![
                "/// Initializes the dart api.",
                "///",
                "/// Is safe to be called multiple times and form multiple",
                "/// thread.",
                "///",
                "/// # Safety",
                "///",
                "/// Must be called with a pointer produced by dart using",
                "/// `NativeApi.initializeApiDLData`.",
            ],
            "int",
            "async_bindgen_dart_init_api__async_api",
            vec!["ffi.Pointer<ffi.Void> init_data,"],
        );

        test_match(
            captures.next().unwrap(),
            vec!["/// Wrapper for initiating the call to an async function."],
            "int",
            "async_bindgen_dart_call__async_api__foo_bar",
            vec![
                "int x,",
                "double y,",
                "int async_bindgen_dart_port_id,",
                "int async_bindgen_dart_completer_id,",
            ],
        );

        test_match(
            captures.next().unwrap(),
            vec![
                "/// Extern \"C\"  wrapper delegating to `PreparedCompleter::extract_result()`.",
                "///",
                "/// # Safety",
                "///",
                "/// See the language specific version of `PreparedCompleter::extract_result()`.",
            ],
            "ffi.Pointer<MyType>",
            "async_bindgen_dart_return__async_api__foo_bar",
            vec!["int handle,"],
        );

        test_match(
            captures.next().unwrap(),
            vec!["/// Wrapper for initiating the call to an async function."],
            "int",
            "async_bindgen_dart_call__async_api__sub",
            vec![
                "int x,",
                "int y,",
                "int async_bindgen_dart_port_id,",
                "int async_bindgen_dart_completer_id,",
            ],
        );

        test_match(
            captures.next().unwrap(),
            vec![
                "/// Extern \"C\"  wrapper delegating to `PreparedCompleter::extract_result()`.",
                "///",
                "/// # Safety",
                "///",
                "/// See the language specific version of `PreparedCompleter::extract_result()`.",
            ],
            "int",
            "async_bindgen_dart_return__async_api__sub",
            vec!["int handle,"],
        );

        test_match(
            captures.next().unwrap(),
            vec![
                "/// Initializes the dart api.",
                "///",
                "/// Is safe to be called multiple times and form multiple",
                "/// thread.",
                "///",
                "/// # Safety",
                "///",
                "/// Must be called with a pointer produced by dart using",
                "/// `NativeApi.initializeApiDLData`.",
            ],
            "int",
            "async_bindgen_dart_init_api__api2",
            vec!["ffi.Pointer<ffi.Void> init_data,"],
        );

        test_match(
            captures.next().unwrap(),
            vec!["/// Wrapper for initiating the call to an async function."],
            "int",
            "async_bindgen_dart_call__api2__get_the_byte",
            vec![
                "int async_bindgen_dart_port_id,",
                "int async_bindgen_dart_completer_id,",
            ],
        );

        test_match(
            captures.next().unwrap(),
            vec![
                "/// Extern \"C\"  wrapper delegating to `PreparedCompleter::extract_result()`.",
                "///",
                "/// # Safety",
                "///",
                "/// See the language specific version of `PreparedCompleter::extract_result()`.",
            ],
            "int",
            "async_bindgen_dart_return__api2__get_the_byte",
            vec!["int handle,"],
        );

        fn test_match(
            captures: Captures,
            doc_comments: Vec<&str>,
            output: &str,
            name: &str,
            inputs: Vec<&str>,
        ) {
            let found_doc_comments =
                captures_as_trimmed_lines(&captures, "doc").collect::<Vec<_>>();
            assert_eq!(found_doc_comments, doc_comments);
            assert_eq!(captures.name("func_name").unwrap().as_str().trim(), name);
            assert_eq!(captures.name("output").unwrap().as_str().trim(), output);
            let found_args = captures_as_trimmed_lines(&captures, "inputs").collect::<Vec<_>>();
            assert_eq!(found_args, inputs);
        }
    }
}
