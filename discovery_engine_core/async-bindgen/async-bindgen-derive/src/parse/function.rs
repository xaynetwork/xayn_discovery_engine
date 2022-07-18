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

use syn::{spanned::Spanned, Error, FnArg, Ident, Signature, Type};

/// Simplified version of `syn::Signature`.
pub(crate) struct FunctionInfo {
    name: Ident,
    output: Type,
    inputs: Vec<FunctionInput>,
}

impl FunctionInfo {
    pub(crate) fn name(&self) -> &Ident {
        &self.name
    }

    pub(crate) fn output(&self) -> &Type {
        &self.output
    }

    pub(crate) fn inputs(&self) -> &[FunctionInput] {
        &self.inputs
    }

    pub(crate) fn from_signature(sig: &Signature) -> Result<Self, Error> {
        if sig.asyncness.is_none() || !sig.generics.params.is_empty() || sig.variadic.is_some() {
            return Err(Error::new(
                sig.ident.span(),
                "async bindgen only works with non-generic, non-variadic async functions",
            ));
        }
        let name = sig.ident.clone();
        let output = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
        };
        let inputs =
            sig.inputs
                .iter()
                .map(|input| match input {
                    FnArg::Receiver(r) => Err(Error::new(
                        r.span(),
                        "`self` is not supported by async bindgen",
                    )),
                    FnArg::Typed(arg) => {
                        let name = match &*arg.pat {
                            syn::Pat::Ident(name) => name.ident.clone(),
                            _ => return Err(Error::new(
                                arg.span(),
                                "patterns in function arguments are not supported by async bindgen",
                            )),
                        };
                        if name.to_string().starts_with("async_bindgen_") {
                            return Err(Error::new(
                                name.span(),
                                "the async_bindgen_ prefix must not be used for arguments",
                            ));
                        }
                        let r#type = (*arg.ty).clone();
                        Ok(FunctionInput { name, r#type })
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?;

        let output = output.unwrap_or_else(|| syn::parse_quote!(()));

        Ok(FunctionInfo {
            name,
            output,
            inputs,
        })
    }
}

pub(crate) struct FunctionInput {
    name: Ident,
    r#type: Type,
}

impl FunctionInput {
    pub(crate) fn new(name: Ident, r#type: Type) -> Self {
        Self { name, r#type }
    }
    pub(crate) fn name(&self) -> &Ident {
        &self.name
    }

    pub(crate) fn r#type(&self) -> &Type {
        &self.r#type
    }
}
