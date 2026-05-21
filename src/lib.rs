#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
  parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Error, Path, Token, WherePredicate,
};

mod attrs;
mod codegen;

use attrs::ContainerAttrs;

/// Derive a native [`quickcheck::Arbitrary`] implementation.
///
/// See the crate-level documentation and `README.md` for the supported
/// `#[quickcheck(...)]` attributes.
#[proc_macro_derive(Arbitrary, attributes(quickcheck))]
pub fn derive_arbitrary(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  expand(input)
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
  let container = ContainerAttrs::parse(&input.attrs)?;
  let qc = container.crate_path();

  let name = &input.ident;
  let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
  let where_clause = build_where_clause(&input, &container, &qc);

  let (arbitrary_body, shrink_body) = match &input.data {
    Data::Struct(data) => codegen::struct_bodies(name, &data.fields, &container, &qc)?,
    Data::Enum(data) => codegen::enum_bodies(name, data, &container, &qc)?,
    Data::Union(_) => {
      return Err(Error::new_spanned(
        &input,
        "`#[derive(Arbitrary)]` does not support unions",
      ))
    }
  };

  Ok(quote! {
    const _: () = {
      impl #impl_generics #qc::Arbitrary for #name #ty_generics #where_clause {
        fn arbitrary(g: &mut #qc::Gen) -> Self {
          #arbitrary_body
        }

        fn shrink(&self) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = Self>> {
          #shrink_body
        }
      }
    };
  })
}

/// Build the `where` clause for the impl.
///
/// * If any container `bound` attr is present, the clause is the type's own
///   predicates plus exactly the parsed bound predicates.
/// * Otherwise, infer `Param: #qc::Arbitrary` for every generic **type** param
///   (lifetimes and const params are skipped).
fn build_where_clause(input: &DeriveInput, container: &ContainerAttrs, qc: &Path) -> TokenStream2 {
  let mut predicates: Punctuated<WherePredicate, Token![,]> = input
    .generics
    .where_clause
    .as_ref()
    .map(|w| w.predicates.clone())
    .unwrap_or_default();

  if container.bounds.is_empty() {
    for type_param in input.generics.type_params() {
      let ident = &type_param.ident;
      predicates.push(syn::parse_quote!(#ident: #qc::Arbitrary));
    }
  } else {
    for predicate in &container.bounds {
      predicates.push(predicate.clone());
    }
  }

  if predicates.is_empty() {
    quote!()
  } else {
    quote!(where #predicates)
  }
}
