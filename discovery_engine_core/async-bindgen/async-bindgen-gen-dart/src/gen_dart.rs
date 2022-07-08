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

use std::{collections::HashMap, io::Write, path::Path};

use anyhow::Error;
use handlebars::Handlebars;
use heck::ToUpperCamelCase;
use once_cell::sync::Lazy;
use serde::Serialize;

use super::parse_genesis::AsyncFunctionSignature;

static IMPORTS_TEMPLATE_STR: &str = r"
import 'dart:ffi' show NativeApi;
import 'dart:ffi' as ffi;

import 'package:async_bindgen_dart_utils/async_bindgen_dart_utils.dart'
    show CouldNotInitializeDartApiError, FfiCompleterRegistry;
// ignore: always_use_package_imports
import '{{ffi_class_import}}';
";

static CLASS_TEMPLATE_STR: &str = r"
class {{type_name}} {
    final {{ffi_class}} _inner;

    {{type_name}}(this._inner) {
        final status = _inner.{{init_function_name}}(NativeApi.initializeApiDLData);
        if (status != 1) {
            throw CouldNotInitializeDartApiError();
        }
    }

    {{#each functions}}
        Future<{{output}}> {{name}}(
            {{#each inputs}}
                {{type}} {{name}},
            {{/each}}
        ) {
            final setup = FfiCompleterRegistry.newCompleter(
                extractor: _inner.{{ffi_return_name}},
            );
            final callOk = _inner.{{ffi_call_name}}(
                {{#each inputs}}
                    {{name}},
                {{/each}}
                setup.portId,
                setup.completerId,
            );
            if (callOk == 0) {
                //TODO
                throw Exception('failed to setup callbacks');
            }
            return setup.future;
        }
    {{/each}}
}
";

const CLASS_TEMPLATE_NAME: &str = "class";
const IMPORTS_TEMPLATE_NAME: &str = "imports";
static TEMPLATES: Lazy<Handlebars> = Lazy::new(|| {
    let mut reg = Handlebars::new();
    reg.register_escape_fn(|v| v.into());
    reg.register_template_string(CLASS_TEMPLATE_NAME, CLASS_TEMPLATE_STR)
        .unwrap();
    reg.register_template_string(IMPORTS_TEMPLATE_NAME, IMPORTS_TEMPLATE_STR)
        .unwrap();
    reg
});

pub(crate) fn generate(
    rel_path: &Path,
    ffi_class: &str,
    module_to_functions: &HashMap<String, Vec<AsyncFunctionSignature>>,
    out: &mut impl Write,
) -> Result<(), Error> {
    let ffi_class_import = &rel_path.display().to_string();
    TEMPLATES.render_to_write(
        IMPORTS_TEMPLATE_NAME,
        &ImportsContext {
            ffi_class,
            ffi_class_import,
        },
        &mut *out,
    )?;

    for (mod_name, functions) in module_to_functions {
        let init_function_name = &format!("async_bindgen_dart_init_api__{}", mod_name);
        let type_name = &mod_name.to_upper_camel_case();

        TEMPLATES.render_to_write(
            CLASS_TEMPLATE_NAME,
            &ClassContext {
                ffi_class,
                functions,
                init_function_name,
                type_name,
            },
            &mut *out,
        )?;
    }
    Ok(())
}

#[derive(Serialize)]
struct ImportsContext<'a> {
    ffi_class: &'a str,
    ffi_class_import: &'a str,
}

#[derive(Serialize)]
struct ClassContext<'a> {
    ffi_class: &'a str,
    functions: &'a [AsyncFunctionSignature],
    init_function_name: &'a str,
    type_name: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::{parse_genesis::DartFunctionInputs, test_utils::assert_trimmed_line_eq};

    use super::*;

    #[test]
    fn test_rendering_template_works() {
        let mut functions = HashMap::<String, Vec<AsyncFunctionSignature>>::default();
        functions.insert(
            "foo_bar".to_owned(),
            vec![
                AsyncFunctionSignature {
                    doc: vec![],
                    name: "func1".into(),
                    ffi_call_name: "c_foobar_func1".into(),
                    ffi_return_name: "r_foobar_func1".into(),
                    output: "int".into(),
                    inputs: vec![
                        DartFunctionInputs {
                            name: "foo".into(),
                            r#type: "ffi.Pointer<Dodo>".into(),
                        },
                        DartFunctionInputs {
                            name: "bar".into(),
                            r#type: "double".into(),
                        },
                    ],
                },
                AsyncFunctionSignature {
                    doc: vec![],
                    name: "d1".into(),
                    ffi_call_name: "foobar_d1c".into(),
                    ffi_return_name: "foobar_d1r".into(),
                    output: "ffi.Pointer<AStruct>".into(),
                    inputs: vec![],
                },
            ],
        );
        let mut out = Vec::<u8>::new();
        generate(
            Path::new("./buddy.dodo.ffigen.dart"),
            "XainFfi",
            &functions,
            &mut out,
        )
        .unwrap();

        assert_trimmed_line_eq!(
            String::from_utf8(out).unwrap(),
            "
            import 'dart:ffi' show NativeApi;
            import 'dart:ffi' as ffi;

            import 'package:async_bindgen_dart_utils/async_bindgen_dart_utils.dart'
                show CouldNotInitializeDartApiError, FfiCompleterRegistry;
            // ignore: always_use_package_imports
            import './buddy.dodo.ffigen.dart';

            class FooBar {
                final XainFfi _inner;

                FooBar(this._inner) {
                    final status = _inner.async_bindgen_dart_init_api__foo_bar(NativeApi.initializeApiDLData);
                    if (status != 1) {
                        throw CouldNotInitializeDartApiError();
                    }
                }

                Future<int> func1(
                    ffi.Pointer<Dodo> foo,
                    double bar,
                ) {
                    final setup = FfiCompleterRegistry.newCompleter(
                        extractor: _inner.r_foobar_func1,
                    );
                    final callOk = _inner.c_foobar_func1(
                            foo,
                            bar,
                        setup.portId,
                        setup.completerId,
                    );
                    if (callOk == 0) {
                        //TODO
                        throw Exception('failed to setup callbacks');
                    }
                    return setup.future;
                }
                Future<ffi.Pointer<AStruct>> d1(
                ) {
                    final setup = FfiCompleterRegistry.newCompleter(
                        extractor: _inner.foobar_d1r,
                    );
                    final callOk = _inner.foobar_d1c(
                        setup.portId,
                        setup.completerId,
                    );
                    if (callOk == 0) {
                        //TODO
                        throw Exception('failed to setup callbacks');
                    }
                    return setup.future;
                }
            }
        "
        );
    }
}
