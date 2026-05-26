//! `#[quickcheck_richderive::quickcheck]` proc-macro-attribute.
//!
//! Sibling to the `#[derive(Arbitrary)]` macro: where the derive lets you
//! attach `arbitrary = "path"` overrides to a *type*'s fields, this attribute
//! lets you attach the same kind of override to a *test function*'s
//! arguments — plus per-test runner config (`cases`, `max_tests`, `gen_size`,
//! `min_tests_passed`).
//!
//! See `README.md` for the full surface and examples; see `parse` and
//! `codegen` for the parser and expansion respectively.

use proc_macro2::TokenStream as TokenStream2;
use syn::ItemFn;

pub(crate) mod codegen;
pub(crate) mod parse;

/// Expand `#[quickcheck_richderive::quickcheck(args)] fn ...`.
///
/// Returns the generated `#[test] fn ...` token stream, or a `syn::Error`
/// for parse errors / unsupported signatures.
pub(crate) fn expand(args: TokenStream2, item: ItemFn) -> syn::Result<TokenStream2> {
  codegen::expand(args, item)
}
