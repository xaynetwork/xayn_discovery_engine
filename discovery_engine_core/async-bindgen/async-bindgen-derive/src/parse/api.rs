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

use heck::ToSnakeCase;
use proc_macro2::{Ident, TokenStream};
use syn::{
    parse::Error,
    spanned::Spanned,
    ImplItem,
    ItemImpl,
    Path,
    PathArguments,
    Type,
    Visibility,
};

use super::function::FunctionInfo;

pub(crate) struct Api {
    type_name: Ident,
    mod_name: Ident,
    functions: Vec<FunctionInfo>,
    header_code: TokenStream,
}

impl Api {
    pub(crate) fn parse(attrs: TokenStream, impl_block: TokenStream) -> Result<Self, Error> {
        let (type_name, functions) = parse_impl_block(impl_block)?;
        let mod_name = Ident::new(&type_name.to_string().to_snake_case(), type_name.span());
        Ok(Self {
            type_name,
            mod_name,
            functions,
            // Include tokens as code to allow injecting imports
            // this is not API forward compatible, but an API braking change
            // is (for now) ok.
            header_code: attrs,
        })
    }

    pub(crate) fn type_name(&self) -> &Ident {
        &self.type_name
    }

    pub(crate) fn mod_name(&self) -> &Ident {
        &self.mod_name
    }

    pub(crate) fn functions(&self) -> &[FunctionInfo] {
        &self.functions
    }

    pub(crate) fn header_code(&self) -> &TokenStream {
        &self.header_code
    }
}

fn parse_impl_block(impl_block: TokenStream) -> Result<(Ident, Vec<FunctionInfo>), Error> {
    let ast = syn::parse2::<ItemImpl>(impl_block)?;
    let name = if let Type::Path(path) = &*ast.self_ty {
        (path.qself.is_none()).then(|| path)
    } else {
        None
    };

    if name.is_none()
        || ast.defaultness.is_some()
        || ast.unsafety.is_some()
        || !ast.generics.params.is_empty()
        || ast.generics.where_clause.is_some()
        || ast.trait_.is_some()
    {
        return Err(Error::new(
            ast.span(),
            "expect a simple `impl <name> {` block",
        ));
    }

    let name = expect_single_seg_path(&name.unwrap().path)?;
    let functions = ast
        .items
        .iter()
        .map(parse_impl_block_fn)
        .collect::<Result<_, _>>()?;
    Ok((name, functions))
}

fn parse_impl_block_fn(item: &ImplItem) -> Result<FunctionInfo, Error> {
    if let ImplItem::Method(method) = item {
        if !matches!(&method.vis, Visibility::Public(_)) || method.defaultness.is_some() {
            Err(Error::new(item.span(), "only async methods for which we should generate bindings are allowed in this impl block"))
        } else {
            FunctionInfo::from_signature(&method.sig)
        }
    } else {
        Err(Error::new(
            item.span(),
            "only methods are allowed in this impl block",
        ))
    }
}

fn expect_single_seg_path(path: &Path) -> Result<Ident, Error> {
    if path.segments.len() == 1 {
        let seg = path.segments.first().unwrap();
        if let PathArguments::None = &seg.arguments {
            return Ok(seg.ident.clone());
        }
    }
    Err(Error::new(path.span(), "expected single ident path"))
}
