#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
  parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Error, Ident, Path, Token, Type,
  WherePredicate,
};

mod attrs;
mod codegen;

use attrs::{ContainerAttrs, FieldAttrs, VariantAttrs};

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

  // Validate *every* field's and variant's attributes upfront, regardless of
  // which codegen path is later taken, so unknown/conflicting attrs are rejected
  // even in positions a particular path would otherwise skip.
  validate_all_attrs(&input)?;

  // The hygienic `&mut Gen` parameter ident, shared by the impl signature and
  // every generated body so they agree even if the user has a `const g` param.
  let g = Ident::new("__quickcheck_g", Span::call_site());

  let name = &input.ident;
  let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
  let where_clause = build_where_clause(&input, &container, &qc);

  let (arbitrary_body, shrink_body) = match &input.data {
    Data::Struct(data) => codegen::struct_bodies(name, &data.fields, &container, &g, &qc)?,
    Data::Enum(data) => codegen::enum_bodies(name, data, &container, &g, &qc)?,
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
        fn arbitrary(#g: &mut #qc::Gen) -> Self {
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
///   predicates plus exactly the parsed bound predicates (wholesale replacement
///   of inference).
/// * Otherwise, infer `Param: #qc::Arbitrary` only for type params that are
///   structurally used by a field actually generated via
///   `<qc>::Arbitrary::arbitrary` (lifetimes and const params are never bounded).
fn build_where_clause(input: &DeriveInput, container: &ContainerAttrs, qc: &Path) -> TokenStream2 {
  let mut predicates: Punctuated<WherePredicate, Token![,]> = input
    .generics
    .where_clause
    .as_ref()
    .map(|w| w.predicates.clone())
    .unwrap_or_default();

  if container.bounds.is_empty() {
    for ident in inferred_bound_params(input, container) {
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

/// The set of generic type-param idents that should receive an inferred
/// `: Arbitrary` bound: those structurally used by some field that is generated
/// via `<qc>::Arbitrary::arbitrary` (see the per-path rules below).
///
/// Returns idents in declaration order. Parse errors are ignored here — they are
/// surfaced by [`validate_all_attrs`] / codegen, so this best-effort inference
/// simply contributes nothing for an unparsable field.
fn inferred_bound_params(input: &DeriveInput, container: &ContainerAttrs) -> Vec<Ident> {
  let type_param_idents: Vec<Ident> = input
    .generics
    .type_params()
    .map(|p| p.ident.clone())
    .collect();
  if type_param_idents.is_empty() {
    return Vec::new();
  }

  // Collect the `syn::Type`s of every field that is generated via `arbitrary`.
  let mut generated_tys: Vec<&Type> = Vec::new();

  // A container `with` builds the whole value itself ⇒ no field uses `arbitrary`.
  if container.with.is_none() {
    match &input.data {
      Data::Struct(data) => collect_arbitrary_field_types(&data.fields, &mut generated_tys),
      Data::Enum(data) => {
        for variant in &data.variants {
          let vattrs = match VariantAttrs::parse(&variant.attrs) {
            Ok(v) => v,
            Err(_) => continue,
          };
          // `skip` variants are never generated; variant `with` builds the
          // whole value ⇒ neither contributes field bounds.
          if vattrs.skip || vattrs.with.is_some() {
            continue;
          }
          collect_arbitrary_field_types(&variant.fields, &mut generated_tys);
        }
      }
      Data::Union(_) => {}
    }
  }

  type_param_idents
    .into_iter()
    .filter(|ident| generated_tys.iter().any(|ty| type_uses_param(ty, ident)))
    .collect()
}

/// Push the `syn::Type` of each field in `fields` that is generated via
/// `Arbitrary::arbitrary` (i.e. no field `with` and not `default`).
fn collect_arbitrary_field_types<'a>(fields: &'a syn::Fields, out: &mut Vec<&'a Type>) {
  for field in fields.iter() {
    let attrs = match FieldAttrs::parse(&field.attrs) {
      Ok(a) => a,
      Err(_) => continue,
    };
    if attrs.with.is_none() && !attrs.default {
      out.push(&field.ty);
    }
  }
}

/// Whether the type-param `ident` appears anywhere within `ty`.
///
/// Walks the type's token stream (recursing into delimited groups) and matches
/// any identifier token equal to `ident`. This is conservative — a same-named
/// associated path segment would also match — but only ever *adds* a sound
/// `: Arbitrary` bound, never drops a required one.
fn type_uses_param(ty: &Type, ident: &Ident) -> bool {
  fn tokens_contain_ident(tokens: TokenStream2, ident: &Ident) -> bool {
    use proc_macro2::TokenTree;
    tokens.into_iter().any(|tt| match tt {
      TokenTree::Ident(i) => &i == ident,
      TokenTree::Group(g) => tokens_contain_ident(g.stream(), ident),
      _ => false,
    })
  }
  tokens_contain_ident(quote!(#ty), ident)
}

/// Validate *every* field's and variant's `#[quickcheck(...)]` attributes so
/// unknown/conflicting attrs are rejected regardless of the codegen path later
/// taken (container `with`, variant `with`, `skip`, etc. would otherwise skip
/// the relevant `parse_*` call). Validation only — results are discarded.
fn validate_all_attrs(input: &DeriveInput) -> syn::Result<()> {
  match &input.data {
    Data::Struct(data) => {
      for field in data.fields.iter() {
        FieldAttrs::parse(&field.attrs)?;
      }
    }
    Data::Enum(data) => {
      for variant in &data.variants {
        VariantAttrs::parse(&variant.attrs)?;
        for field in variant.fields.iter() {
          FieldAttrs::parse(&field.attrs)?;
        }
      }
    }
    Data::Union(_) => {}
  }
  Ok(())
}
