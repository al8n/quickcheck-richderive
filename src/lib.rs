#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
  Data, DeriveInput, Error, Ident, ItemFn, Path, Token, Type, WherePredicate, parse_macro_input,
  punctuated::Punctuated,
};

mod attrs;
mod codegen;
mod test_attr;

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

/// Proptest-style `#[quickcheck]` attribute with per-arg generator
/// overrides (via `#[strategy(...)]` on the fn parameters) and per-test
/// config.
///
/// Drops in as a replacement for `#[quickcheck_macros::quickcheck]` (same
/// attribute name on purpose). The bare form `#[quickcheck_richderive::quickcheck]`
/// is behaviour-identical to that vanilla attribute (each arg uses its type's
/// `Arbitrary` impl); attribute arguments unlock runner tuning and the
/// `crate = "..."` path override.
///
/// ```ignore
/// use quickcheck_richderive::quickcheck;
///
/// fn small_positive(g: &mut ::quickcheck::Gen) -> i32 {
///     (u32::arbitrary(g) % 100) as i32 + 1
/// }
///
/// #[quickcheck(cases = 1000)]
/// fn round_trip(
///     #[strategy(small_positive)] a: i32,
///     b: String,
/// ) -> bool {
///     encode(&a, &b).decode() == (a, b)
/// }
/// ```
///
/// See the README's `#[quickcheck]` attribute section for the full key
/// reference, `#[strategy(...)]` semantics, the `prop_assert!` family, and
/// the `crate = "..."` knob.
#[proc_macro_attribute]
pub fn quickcheck(args: TokenStream, item: TokenStream) -> TokenStream {
  let args = TokenStream2::from(args);
  let item = parse_macro_input!(item as ItemFn);
  test_attr::expand(args, item)
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

  // Allocates all macro-internal identifiers so none collides with a user
  // `const` parameter of the same spelling (which bypasses macro hygiene).
  let hyg = codegen::Hygiene::new(&input);

  // The `&mut Gen` parameter ident, shared by the impl signature and every
  // generated body (collision-free + `mixed_site`).
  let g = hyg.ident("__quickcheck_g");

  // `Box` for `shrink`'s return: an explicit `box = "..."`, or the feature
  // default. `box_prelude` carries the `extern crate alloc` alias for the
  // self-contained alloc default (empty otherwise).
  let (box_prelude, box_ty) = box_setup(&container, &hyg)?;

  let name = &input.ident;
  let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
  let where_clause = build_where_clause(&input, &container, &qc);

  let (arbitrary_body, shrink_body) = match &input.data {
    Data::Struct(data) => {
      codegen::struct_bodies(name, &data.fields, &container, &g, &hyg, &box_ty, &qc)?
    }
    Data::Enum(data) => codegen::enum_bodies(name, data, &container, &g, &hyg, &box_ty, &qc)?,
    Data::Union(_) => {
      return Err(Error::new_spanned(
        &input,
        "`#[derive(Arbitrary)]` does not support unions",
      ));
    }
  };

  Ok(quote! {
    const _: () = {
      #box_prelude
      // `#[automatically_derived]` marks this as derive output (and is exempt
      // from `non_local_definitions`); the `allow` is defensive across rustc
      // versions, with `unknown_lints` keeping it valid on toolchains predating
      // the lint.
      #[automatically_derived]
      #[allow(unknown_lints, non_local_definitions)]
      impl #impl_generics #qc::Arbitrary for #name #ty_generics #where_clause {
        fn arbitrary(#g: &mut #qc::Gen) -> Self {
          #arbitrary_body
        }

        fn shrink(&self) -> #box_ty<dyn ::core::iter::Iterator<Item = Self>> {
          #shrink_body
        }
      }
    };
  })
}

/// `Box` selection for `shrink`'s return type, returned as
/// `(prelude, box_path)`: `prelude` is injected at the top of the generated
/// `const` block (empty except for the alloc default) and `box_path` is the
/// `Box` type.
///
/// An explicit container `box = "..."` always wins (and is the only
/// per-invocation, unification-immune selector). Otherwise the choice comes from
/// this crate's features — but note Cargo **unifies** features and a proc-macro
/// is compiled **once**, so the feature default is workspace-global, not
/// per-consumer. `std` takes precedence when both are active; with neither, we
/// error rather than silently guess.
///
/// The alloc default is **self-contained**: it aliases the always-available
/// `alloc` sysroot crate *inside* the generated `const` block
/// (`extern crate alloc as <alias>;`) so a `#![no_std]` consumer need not declare
/// `extern crate alloc;` itself (a bare `::alloc` path would fail with E0433).
/// The alias goes through `hyg` so it can't collide with a user generic named
/// like it. An explicit `box = "..."` path, by contrast, is emitted verbatim —
/// the consumer is responsible for its resolvability (e.g. `box = "alloc::..."`
/// needs their own `extern crate alloc;`).
fn box_setup(
  container: &ContainerAttrs,
  _hyg: &codegen::Hygiene,
) -> syn::Result<(TokenStream2, Path)> {
  if let Some(p) = &container.box_path {
    return Ok((TokenStream2::new(), p.clone()));
  }
  #[cfg(feature = "std")]
  {
    Ok((TokenStream2::new(), syn::parse_quote!(::std::boxed::Box)))
  }
  #[cfg(all(not(feature = "std"), feature = "alloc"))]
  {
    let alias = _hyg.ident("__quickcheck_alloc");
    Ok((
      quote!(extern crate alloc as #alias;),
      syn::parse_quote!(#alias::boxed::Box),
    ))
  }
  #[cfg(all(not(feature = "std"), not(feature = "alloc")))]
  {
    Err(syn::Error::new(
      proc_macro2::Span::call_site(),
      "quickcheck-derive: no `Box` type available — enable the `std` (default) or \
       `alloc` feature, or set a container `#[quickcheck(box = \"...\")]`",
    ))
  }
}

/// Build the `where` clause for the impl.
///
/// * If any container `bound` attr is present, the clause is the type's own
///   predicates plus exactly the parsed bound predicates (wholesale replacement
///   of inference).
/// * Otherwise infer three kinds of predicate:
///   - `Self: Clone + 'static` (the `Arbitrary` supertrait obligation, stated
///     exactly on the implementing type);
///   - `<FieldTy>: #qc::Arbitrary` for each `Arbitrary`-generated field type that
///     mentions a generic param (sound for projections / nested generics); and
///   - `<FieldTy>: Default` for each `#[quickcheck(default)]` field type that
///     mentions a generic param (the generated `Default::default()` needs it).
fn build_where_clause(input: &DeriveInput, container: &ContainerAttrs, qc: &Path) -> TokenStream2 {
  let mut predicates: Punctuated<WherePredicate, Token![,]> = input
    .generics
    .where_clause
    .as_ref()
    .map(|w| w.predicates.clone())
    .unwrap_or_default();

  if container.bounds.is_empty() {
    // State the `Arbitrary: Clone + 'static` supertrait obligation exactly, on
    // the implementing type itself — not approximated as `T: Clone + 'static`
    // per param (which would over-constrain manual-`Clone` types and miss
    // lifetime outlives bounds). `Self` resolves to the implementing type.
    if !input.generics.params.is_empty() {
      predicates.push(syn::parse_quote!(Self: ::core::clone::Clone + 'static));
    }
    // Inline-generated fields call `Arbitrary::arbitrary`; `default` fields call
    // `Default::default()`. `with`/`arbitrary` fields call user code instead, so
    // they need neither bound. Bound each accordingly.
    for ty in inferred_field_types(input, container, |a| {
      a.with.is_none() && a.arbitrary.is_none() && !a.default
    }) {
      predicates.push(syn::parse_quote!(#ty: #qc::Arbitrary));
    }
    for ty in inferred_field_types(input, container, |a| a.default) {
      predicates.push(syn::parse_quote!(#ty: ::core::default::Default));
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

/// The set of **field types** that should receive an inferred `: Arbitrary`
/// bound: the types of fields generated via `<qc>::Arbitrary::arbitrary` that
/// mention at least one generic type param.
///
/// Bounding the field type itself (e.g. `<T as Trait>::Item: Arbitrary`), rather
/// than the type params found inside it (`T: Arbitrary`), is sound for projected
/// / associated types — where `T: Arbitrary` would *not* imply the projection is
/// `Arbitrary` — and for nested generics (`Vec<T>: Arbitrary`). Concrete field
/// types (mentioning no param) are left to the call site, so we don't emit
/// redundant `u32: Arbitrary`-style bounds.
///
/// A field type is considered to "mention a generic param" if it references any
/// generic **type or const** param — so const-generic-bearing field types
/// (`Only<N>`, `[u8; N]`) are bounded too, not just type-param ones.
///
/// `select` picks which fields contribute (by their parsed attrs), so the same
/// walk serves both the `Arbitrary` and the `Default` bound passes. Only fields
/// generated inline contribute: nothing under a container `with`, and for enums
/// only non-`skip` variants without a variant `with`.
///
/// Deduplicated, in first-seen order. Parse errors are ignored here — they are
/// surfaced by [`validate_all_attrs`] / codegen, so this best-effort inference
/// simply contributes nothing for an unparsable field.
fn inferred_field_types(
  input: &DeriveInput,
  container: &ContainerAttrs,
  select: fn(&FieldAttrs) -> bool,
) -> Vec<Type> {
  // Generic type *and* const params (lifetimes never need a bound here).
  let param_idents: Vec<Ident> = input
    .generics
    .type_params()
    .map(|p| p.ident.clone())
    .chain(input.generics.const_params().map(|p| p.ident.clone()))
    .collect();
  if param_idents.is_empty() {
    return Vec::new();
  }

  let mut tys: Vec<Type> = Vec::new();

  // A container `with`/`arbitrary` builds the whole value itself ⇒ no field is
  // generated.
  if container.with.is_none() && container.arbitrary.is_none() {
    match &input.data {
      Data::Struct(data) => collect_field_types(&data.fields, select, &mut tys),
      Data::Enum(data) => {
        for variant in &data.variants {
          let vattrs = match VariantAttrs::parse(&variant.attrs) {
            Ok(v) => v,
            Err(_) => continue,
          };
          // `skip` variants are never generated; a variant `with`/`arbitrary`
          // builds the whole value ⇒ none contribute field bounds.
          if vattrs.skip || vattrs.with.is_some() || vattrs.arbitrary.is_some() {
            continue;
          }
          collect_field_types(&variant.fields, select, &mut tys);
        }
      }
      Data::Union(_) => {}
    }
  }

  let mut seen = std::collections::HashSet::new();
  tys
    .into_iter()
    .filter(|ty| param_idents.iter().any(|p| type_uses_param(ty, p)))
    .filter(|ty| seen.insert(quote!(#ty).to_string()))
    .collect()
}

/// Push the `syn::Type` of each field in `fields` whose parsed attrs satisfy
/// `select`.
fn collect_field_types(fields: &syn::Fields, select: fn(&FieldAttrs) -> bool, out: &mut Vec<Type>) {
  for field in fields.iter() {
    let attrs = match FieldAttrs::parse(&field.attrs) {
      Ok(a) => a,
      Err(_) => continue,
    };
    if select(&attrs) {
      out.push(field.ty.clone());
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
