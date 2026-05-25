//! Parser for `#[quickcheck_richderive::quickcheck(...)]` attribute arguments.
//!
//! The attribute accepts a comma-separated list of `key = value` pairs. Five
//! keys are recognised; every other identifier is rejected. Per-argument
//! generator overrides live on the **fn parameters** via `#[strategy(path)]`,
//! not in this attribute — they are parsed by the codegen step.
//!
//! | Key | Type | Notes |
//! |---|---|---|
//! | `cases = N`              | `u64` literal   | `.tests(N)` on the runner |
//! | `max_tests = N`          | `u64` literal   | `.max_tests(N)` (discard cap) |
//! | `gen_size = N`           | `usize` literal | `Gen::new(N)` |
//! | `min_tests_passed = N`   | `u64` literal   | omitted if unset |
//! | `crate = "path"`         | path string     | base path for `Arbitrary`/`Gen`/`QuickCheck`/`TestResult` (default `::quickcheck`) |
//!
//! Per-argument generator overrides have moved to `#[strategy(path)]` on each
//! fn argument, parsed in the codegen stage.

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{
  Error, Ident, LitInt, LitStr, Path, Token,
  ext::IdentExt,
  parse::{Parse, ParseStream},
  parse_str,
  punctuated::Punctuated,
};

/// Fully-parsed attribute arguments, post-shape-check.
pub(crate) struct TestAttrArgs {
  /// `cases = N` — `.tests(N)`. Default `100`.
  pub(crate) cases: u64,
  /// `max_tests = N` — `.max_tests(N)`. Default `10_000`.
  pub(crate) max_tests: u64,
  /// `gen_size = N` — `Gen::new(N)`. Default `100`.
  pub(crate) gen_size: usize,
  /// `min_tests_passed = N` — `.min_tests_passed(N)`. `None` ⇒ omit the call.
  pub(crate) min_tests_passed: Option<u64>,
  /// `crate = "path"` — base path for `Arbitrary` / `Gen` / `QuickCheck` /
  /// `TestResult` and the injected `prop_assert!` macros. Default `::quickcheck`.
  pub(crate) krate: Option<Path>,
}

impl Default for TestAttrArgs {
  fn default() -> Self {
    // Defaults match `quickcheck::QuickCheck::new()` itself, so the bare-form
    // expansion behaves like `#[quickcheck_macros::quickcheck]`.
    Self {
      cases: 100,
      max_tests: 10_000,
      gen_size: 100,
      min_tests_passed: None,
      krate: None,
    }
  }
}

impl TestAttrArgs {
  /// Parse the attribute's argument tokens. An empty token-stream returns
  /// defaults — the bare `#[quickcheck_richderive::quickcheck]` form.
  pub(crate) fn parse(tokens: TokenStream2) -> syn::Result<Self> {
    if tokens.is_empty() {
      return Ok(Self::default());
    }
    syn::parse2::<Self>(tokens)
  }

  /// The resolved quickcheck base path (default `::quickcheck`).
  pub(crate) fn crate_path(&self) -> Path {
    self
      .krate
      .clone()
      .unwrap_or_else(|| syn::parse_quote!(::quickcheck))
  }
}

impl Parse for TestAttrArgs {
  fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
    let mut out = TestAttrArgs::default();
    // Track which keys we've already seen, by ident name. A duplicate of any
    // key is an error at the *second* occurrence's span (the first is the
    // authoritative value, conceptually).
    let mut seen: HashSet<String> = HashSet::new();

    // Parse `key = value, key = value, ...` until exhaustion. A trailing
    // comma is allowed.
    let entries: Punctuated<KvEntry, Token![,]> = Punctuated::parse_terminated(input)?;

    for entry in entries {
      let key_str = entry.key.to_string();
      if !seen.insert(key_str.clone()) {
        return Err(Error::new(
          entry.key.span(),
          format!("duplicate key `{key_str}` in #[quickcheck_richderive::quickcheck(...)]"),
        ));
      }

      match key_str.as_str() {
        "cases" => out.cases = parse_u64(&entry, "cases")?,
        "max_tests" => out.max_tests = parse_u64(&entry, "max_tests")?,
        "gen_size" => out.gen_size = parse_usize(&entry, "gen_size")?,
        "min_tests_passed" => {
          out.min_tests_passed = Some(parse_u64(&entry, "min_tests_passed")?);
        }
        "crate" => out.krate = Some(expect_path_string(&entry, "crate")?),
        // Any non-reserved key is a user error — per-arg overrides are no
        // longer recognised here (they live on the fn args as
        // `#[strategy(...)]`). Surface a focused, actionable diagnostic.
        _ => {
          return Err(Error::new(
            entry.key.span(),
            format!(
              "unknown key `{key_str}` in #[quickcheck_richderive::quickcheck(...)]; \
               expected one of `cases`, `max_tests`, `gen_size`, `min_tests_passed`, or \
               `crate`. For a per-argument generator, attach `#[strategy(path)]` to the \
               fn parameter instead. (Note: deterministic seeding via `seed` is not \
               supported by upstream `quickcheck`; see the README's reference table.)"
            ),
          ));
        }
      }
    }

    Ok(out)
  }
}

/// One `key = value` pair, value held as raw tokens for the dispatcher to
/// re-interpret based on the key's shape. The `=` token isn't retained; the
/// value's own span is enough for shape-mismatch diagnostics.
struct KvEntry {
  key: Ident,
  value: KvValue,
}

impl Parse for KvEntry {
  fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
    // `parse_any` accepts raw and keyword identifiers — in particular `crate`,
    // which is a reserved word but a valid attribute key (matching the
    // derive's `#[quickcheck(crate = "...")]` convention).
    let key: Ident = Ident::parse_any(input)?;
    let _eq: Token![=] = input.parse()?;
    let value: KvValue = input.parse()?;
    Ok(Self { key, value })
  }
}

/// The right-hand side of a `key = value` entry. We accept exactly two value
/// shapes: integer literals (for runner-config keys) and string literals (for
/// the `crate` path). Mismatches are reported at the value's span by
/// `parse_u64` / `parse_usize` / `expect_path_string`.
enum KvValue {
  Int(LitInt),
  Str(LitStr),
}

impl KvValue {
  fn span(&self) -> Span {
    match self {
      Self::Int(lit) => lit.span(),
      Self::Str(lit) => lit.span(),
    }
  }
}

impl Parse for KvValue {
  fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
    let lookahead = input.lookahead1();
    if lookahead.peek(LitInt) {
      Ok(Self::Int(input.parse()?))
    } else if lookahead.peek(LitStr) {
      Ok(Self::Str(input.parse()?))
    } else {
      Err(lookahead.error())
    }
  }
}

fn parse_u64(entry: &KvEntry, key: &str) -> syn::Result<u64> {
  match &entry.value {
    KvValue::Int(lit) => lit
      .base10_parse::<u64>()
      .map_err(|e| Error::new(lit.span(), format!("`{key}` must fit in `u64`: {e}"))),
    KvValue::Str(_) => Err(Error::new(
      entry.value.span(),
      format!("`{key}` expects an integer literal, not a string"),
    )),
  }
}

fn parse_usize(entry: &KvEntry, key: &str) -> syn::Result<usize> {
  match &entry.value {
    KvValue::Int(lit) => lit
      .base10_parse::<usize>()
      .map_err(|e| Error::new(lit.span(), format!("`{key}` must fit in `usize`: {e}"))),
    KvValue::Str(_) => Err(Error::new(
      entry.value.span(),
      format!("`{key}` expects an integer literal, not a string"),
    )),
  }
}

/// `crate = "..."` expects a *string literal* carrying a `syn::Path` —
/// matching the derive's `#[quickcheck(crate = "...")]` convention.
fn expect_path_string(entry: &KvEntry, key: &str) -> syn::Result<Path> {
  let lit = match &entry.value {
    KvValue::Str(lit) => lit,
    KvValue::Int(_) => {
      return Err(Error::new(
        entry.value.span(),
        format!("`{key}` expects a string literal path (e.g. `{key} = \"::quickcheck\"`)"),
      ));
    }
  };
  parse_str::<Path>(&lit.value()).map_err(|e| Error::new(lit.span(), e))
}
