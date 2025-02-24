/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::collections::HashMap;

use proc_macro2::Ident;
use quote::quote;
use quote::quote_spanned;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Attribute;
use syn::DeriveInput;
use syn::MetaNameValue;
use syn::Token;

const STARLARK_DOCS_ATTRS: &str = "starlark_docs_attrs";

pub fn derive_docs(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_docs_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_docs_derive(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let span = input.span();
    let DeriveInput {
        ident: name,
        generics,
        attrs,
        ..
    } = input;

    let parsed_attrs = parse_custom_attributes(attrs)?;
    let name_str = name.to_string();
    let mod_name = Ident::new(&format!("__starlark_docs_derive_{}", name_str), span);
    // Import whatever "starlark" is (either an extern package, or if this is used *in* the
    // starlark package, the local import of `use crate as starlark`. We can't just use
    // super::starlark, as you'd always have to have that directly imported to use the derive
    // macro, and if we just try to use 'starlark', we get ambiguity compile errors if used
    // within the starlark crate itself. Submodules are hard.
    let starlark_import = Ident::new(&format!("__starlark_docs_import_{}", name_str), span);
    let custom_attrs: Vec<_> = parsed_attrs
        .into_iter()
        .map(|(k, v)| {
            quote! { (#k.to_owned(), #v.to_owned())}
        })
        .collect();

    Ok(quote_spanned! {span=>
        impl #generics #name #generics {
            #[doc(hidden)]
            pub fn __generated_documentation() -> Option<starlark::values::docs::Doc> {
                let name = <#name as starlark::values::StarlarkValue>::get_type_value_static().as_str().to_owned();
                let id = starlark::values::docs::Identifier {
                    name,
                    location: None,
                };
                let item = <#name as starlark::values::StarlarkValue>::get_methods()?.documentation();
                let custom_attrs = std::collections::HashMap::from([
                    #(#custom_attrs),*
                ]);
                Some(starlark::values::docs::Doc {
                    id,
                    item,
                    custom_attrs,
                })
            }
        }

        use starlark as #starlark_import;

        #[allow(non_snake_case)]
        mod #mod_name {
            use super::#starlark_import as starlark;
            use self::starlark::__derive_refs::inventory as inventory;

            inventory::submit! {
                #[allow(unknown_lints)]
                #[allow(gazebo_lint_use_box)]
                self::starlark::values::docs::RegisteredDoc {
                    getter: Box::new(super::#name::__generated_documentation)
                }
            }
        }
    })
}

fn get_attrs(attr: Attribute) -> syn::Result<HashMap<String, String>> {
    let mut found = HashMap::new();
    let args: Punctuated<MetaNameValue, Token![,]> =
        attr.parse_args_with(Punctuated::parse_terminated)?;
    for arg in args {
        match &arg {
            MetaNameValue {
                path,
                lit: syn::Lit::Str(s),
                ..
            } => {
                let ident = path.get_ident().unwrap();
                let attr_name = ident.to_string();
                if found.insert(attr_name, s.value()).is_some() {
                    return Err(syn::Error::new(
                        arg.span(),
                        format!("Argument {} was specified twice", ident),
                    ));
                }
            }
            MetaNameValue { path, .. } => {
                return Err(syn::Error::new(
                    arg.span(),
                    format!(
                        "Argument {} must have a string literal value",
                        path.get_ident().unwrap(),
                    ),
                ));
            }
        }
    }
    Ok(found)
}

fn parse_custom_attributes(attrs: Vec<Attribute>) -> syn::Result<HashMap<String, String>> {
    for attr in attrs {
        if attr.path.is_ident(STARLARK_DOCS_ATTRS) {
            return get_attrs(attr);
        }
    }

    Ok(HashMap::new())
}
