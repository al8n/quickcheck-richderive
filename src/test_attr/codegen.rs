//! Code generation for `#[quickcheck_richderive::test]`.
//!
//! Given the parsed attribute args and the user's `ItemFn`, emit a `#[test]`
//! function that:
//!
//! 1. Re-emits the user's function verbatim as an inner item (so its body,
//!    attrs, generics, and visibility round-trip with zero behaviour change).
//! 2. For each per-arg override, emits a private newtype that implements
//!    `quickcheck::Arbitrary` by calling the user-supplied generator path and
//!    delegating `shrink` to the underlying type — this preserves shrinking
//!    without exposing a `Shrink` knob in the attribute surface.
//! 3. Emits a `__wrapper` fn matching the user's signature on the un-overridden
//!    args and the newtype on overridden args; the wrapper unwraps each
//!    newtype and forwards to the inner fn.
//! 4. Builds `QuickCheck::new().rng(Gen::new(gen_size)).tests(...).max_tests(...)`
//!    [optionally `.min_tests_passed(...)`] and finally `.quickcheck(__wrapper
//!    as fn(...) -> R)`.
//!
//! Identifier hygiene: every generated ident is `mixed_site` and carries a
//! `__qrd_` prefix so it cannot collide with user-supplied idents in the
//! enclosing test function. Per-arg newtype names embed the arg ident
//! verbatim (`__QrdArg_<ident>`) for readability in compiler diagnostics.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
  Error, FnArg, Ident, ItemFn, Pat, PatType, ReturnType, Signature, Type, spanned::Spanned,
};

use crate::test_attr::parse::{ArgOverride, TestAttrArgs};

/// Internal description of one of the user fn's positional arguments after
/// matching it against the parsed attribute overrides.
struct ArgPlan {
  /// The original parameter ident from the user's signature (always present —
  /// patterns are rejected upstream).
  ident: Ident,
  /// Whether the original binding was `mut x: T`. We re-emit the `mut` on
  /// the **inner** fn's parameter so any mutable-rebind in the user's body
  /// still type-checks.
  mutability: bool,
  /// The parameter's declared type (preserved verbatim for the inner fn / the
  /// newtype's inner field type).
  ty: Type,
  /// `Some(path)` if a `<ident> = "path"` override applied; the wrapper
  /// will then box the arg through a generated newtype calling `path(g)`.
  override_path: Option<syn::Path>,
}

/// Main entry point. Returns the generated `#[test]` `fn { ... }` token stream
/// or a `syn::Error` for any user-facing diagnostic.
pub(crate) fn expand(args: TokenStream2, item: ItemFn) -> syn::Result<TokenStream2> {
  let parsed_args = TestAttrArgs::parse(args)?;

  // Reject signature shapes we don't support, with focused spans so the
  // user sees the offending bit and not the whole attribute.
  reject_unsupported_signature(&item.sig)?;

  let plans = build_arg_plans(&item.sig, &parsed_args.arg_overrides)?;

  // Names we generate. Span::mixed_site keeps them invisible to the user
  // body; the `__qrd_` prefix is for human eyes during compiler errors.
  let inner_fn_ident: Ident = Ident::new("__qrd_inner", Span::mixed_site());
  let wrapper_fn_ident: Ident = Ident::new("__qrd_wrapper", Span::mixed_site());
  let g_param: Ident = Ident::new("__qrd_g", Span::mixed_site());

  // Per-override newtype defs + the wrapper-side conversion expression.
  let newtype_defs = plans.iter().filter_map(|p| newtype_def(p, &g_param));
  let newtype_defs: Vec<TokenStream2> = newtype_defs.collect();

  // Wrapper-fn parameter list and the call-into-inner argument list.
  let wrapper_params: Vec<TokenStream2> = plans
    .iter()
    .map(|p| {
      let pname = wrapper_param_ident(&p.ident);
      let pty = wrapper_param_type(p);
      quote!(#pname: #pty)
    })
    .collect();

  let inner_call_args: Vec<TokenStream2> = plans
    .iter()
    .map(|p| {
      let pname = wrapper_param_ident(&p.ident);
      if p.override_path.is_some() {
        // Newtype: unwrap the `.0` (`Clone` already derived so this move is
        // straightforward — we own `pname` by-value into the wrapper).
        quote!(#pname.0)
      } else {
        quote!(#pname)
      }
    })
    .collect();

  // The wrapper's parameter *types* for the `as fn(...) -> R` coercion.
  // Must match `wrapper_params` exactly — but stripped of the param names.
  let wrapper_fn_type_inputs: Vec<TokenStream2> =
    plans.iter().map(wrapper_param_type).collect();

  // The user's return type, preserved verbatim. Default to `()` so the
  // `as fn(...) -> R` coercion is always well-formed.
  let return_ty = match &item.sig.output {
    ReturnType::Default => quote!(()),
    ReturnType::Type(_, ty) => quote!(#ty),
  };

  // The inner fn's parameter list, by-position, with the user's natural types
  // so the body — which references the original idents — compiles unchanged.
  // Preserve `mut` on the binding so `fn f(mut x: u8) { x += 1; }` still
  // type-checks inside the rewritten inner fn.
  let inner_params: Vec<TokenStream2> = plans
    .iter()
    .map(|p| {
      let id = &p.ident;
      let ty = &p.ty;
      if p.mutability {
        quote!(mut #id: #ty)
      } else {
        quote!(#id: #ty)
      }
    })
    .collect();

  // The user's original fn body. We don't textually splice it; we emit the
  // whole `ItemFn` with its name swapped to the inner ident so attributes /
  // generics / where-clauses / unsafety / etc. survive.
  let inner_item = rewrite_to_inner(&item, &inner_fn_ident, &inner_params, &return_ty)?;

  let TestAttrArgs {
    cases,
    max_tests,
    gen_size,
    min_tests_passed,
    ..
  } = parsed_args;

  // Optional `.min_tests_passed(N)` chain.
  let min_chain = match min_tests_passed {
    Some(n) => quote!(.min_tests_passed(#n)),
    None => quote!(),
  };

  // The outer test fn ident: keep the user's name so `cargo test` filters /
  // diagnostics speak the same name they wrote.
  let outer_name = &item.sig.ident;
  // Preserve outer attrs (e.g. `#[ignore]`, `#[cfg(...)]`, `#[should_panic]`)
  // on the generated `#[test]` fn. These need to land on whichever item
  // libtest harvests as the test, which is the outer `#[test]` fn.
  let outer_attrs = &item.attrs;

  // The wrapper forwards directly: each wrapper param is unwrapped (for
  // overridden args, `.0` peels the newtype; otherwise pass-through) and
  // handed straight to the inner fn in positional order.
  Ok(quote! {
    #(#outer_attrs)*
    #[test]
    fn #outer_name() {
      // Re-emit the user's fn verbatim, renamed to the inner ident.
      #inner_item

      // Per-override newtypes (zero of them if no overrides).
      #(#newtype_defs)*

      // The wrapper-fn whose signature is what `.quickcheck()` will run
      // through `Testable`. It accepts the per-arg `Arbitrary` types
      // (overridden or natural) and forwards to `__qrd_inner`.
      fn #wrapper_fn_ident(#(#wrapper_params),*) -> #return_ty {
        #inner_fn_ident(#(#inner_call_args),*)
      }

      ::quickcheck::QuickCheck::new()
        .rng(::quickcheck::Gen::new(#gen_size))
        .tests(#cases)
        .max_tests(#max_tests)
        #min_chain
        .quickcheck(#wrapper_fn_ident as fn(#(#wrapper_fn_type_inputs),*) -> #return_ty);
    }
  })
}

/// Rewrite the user's `ItemFn` to use `inner_ident` as its name and a flat
/// param list `(name: ty, ...)`. We rebuild the signature rather than mutate
/// it in place so any patterns / `mut` bindings that survived
/// `reject_unsupported_signature` become plain `name: ty` bindings — the inner
/// fn callers always pass through the user idents.
fn rewrite_to_inner(
  item: &ItemFn,
  inner_ident: &Ident,
  inner_params: &[TokenStream2],
  return_ty: &TokenStream2,
) -> syn::Result<TokenStream2> {
  // Outer-fn attrs (`#[ignore]`, `#[should_panic]`, ...) belong on the
  // generated `#[test]` fn, not on this nested helper, so we drop them here
  // and re-attach them at the outer site. The inner is a closed-over local
  // — visibility is likewise irrelevant.
  let constness = &item.sig.constness;
  let asyncness = &item.sig.asyncness;
  if asyncness.is_some() {
    return Err(Error::new(
      asyncness.span(),
      "#[quickcheck_richderive::test] does not support `async fn` — \
       quickcheck has no async test runner; remove `async` or wait for upstream support",
    ));
  }
  let unsafety = &item.sig.unsafety;
  if unsafety.is_some() {
    return Err(Error::new(
      unsafety.span(),
      "#[quickcheck_richderive::test] does not support `unsafe fn`",
    ));
  }
  let abi = &item.sig.abi;
  let generics = &item.sig.generics;
  if !generics.params.is_empty() {
    return Err(Error::new(
      generics.span(),
      "#[quickcheck_richderive::test] does not support generic functions",
    ));
  }
  let where_clause = &generics.where_clause;
  let body = &item.block;

  Ok(quote! {
    #constness #asyncness #unsafety #abi fn #inner_ident #generics(#(#inner_params),*) -> #return_ty #where_clause
      #body
  })
}

/// Reject signature shapes that don't survive the wrapper expansion. These
/// errors point at the *offending* tokens, not the whole attribute, so the
/// user sees what to fix.
fn reject_unsupported_signature(sig: &Signature) -> syn::Result<()> {
  if let Some(variadic) = &sig.variadic {
    return Err(Error::new(
      variadic.span(),
      "#[quickcheck_richderive::test] does not support variadic functions",
    ));
  }
  for input in &sig.inputs {
    match input {
      FnArg::Receiver(r) => {
        return Err(Error::new(
          r.span(),
          "#[quickcheck_richderive::test] expects free functions, not methods (no `self`)",
        ));
      }
      FnArg::Typed(PatType { pat, .. }) => {
        // Only plain idents — patterns (`(a, b): (T, U)`, `_`, ...) would
        // need us to invent a name to bind in the wrapper. Cheaper to
        // refuse than to over-engineer the rename.
        match pat.as_ref() {
          Pat::Ident(_) => {}
          _ => {
            return Err(Error::new(
              pat.span(),
              "#[quickcheck_richderive::test] expects each fn argument to be a plain \
               identifier (no patterns); rebind inside the body if you need destructuring",
            ));
          }
        }
      }
    }
  }
  Ok(())
}

/// Build the per-arg plan, matching each parameter against the parsed
/// overrides. Any override naming a non-existent parameter is reported with
/// the *override's* span (since that's the user's typo).
fn build_arg_plans(
  sig: &Signature,
  overrides: &[ArgOverride],
) -> syn::Result<Vec<ArgPlan>> {
  // First, harvest the (ident, ty) pairs from the signature.
  let mut plans: Vec<ArgPlan> = Vec::with_capacity(sig.inputs.len());
  for input in &sig.inputs {
    match input {
      FnArg::Typed(PatType { pat, ty, .. }) => {
        let (ident, mutability) = match pat.as_ref() {
          Pat::Ident(pi) => (pi.ident.clone(), pi.mutability.is_some()),
          // Already rejected by `reject_unsupported_signature`; defensive
          // fallback to keep the error surface honest if anyone ever
          // shuffles the call order.
          other => {
            return Err(Error::new(
              other.span(),
              "internal: non-ident pattern reached arg-plan builder",
            ));
          }
        };
        plans.push(ArgPlan {
          ident,
          mutability,
          ty: (**ty).clone(),
          override_path: None,
        });
      }
      // Already rejected; mirror the diagnostic.
      FnArg::Receiver(r) => {
        return Err(Error::new(
          r.span(),
          "#[quickcheck_richderive::test] expects free functions, not methods",
        ));
      }
    }
  }

  // Now attach overrides. A second override for the same ident is a parser
  // error (caught upstream), so each pass here is unique.
  for ov in overrides {
    let target = plans.iter_mut().find(|p| p.ident == ov.arg);
    match target {
      Some(plan) => plan.override_path = Some(ov.path.clone()),
      None => {
        return Err(Error::new(
          ov.arg.span(),
          format!(
            "`{}` is not a parameter of this function — per-arg overrides must name \
             one of the fn's positional arguments",
            ov.arg
          ),
        ));
      }
    }
  }

  Ok(plans)
}

/// For a per-override arg `a: T`, build a newtype:
///
/// ```ignore
/// #[derive(Clone)]
/// struct __QrdArg_a(T);
/// impl ::core::fmt::Debug for __QrdArg_a { ... }
/// impl ::quickcheck::Arbitrary for __QrdArg_a {
///     fn arbitrary(g: &mut ::quickcheck::Gen) -> Self { Self(<path>(g)) }
///     fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
///         Box::new(<T as ::quickcheck::Arbitrary>::shrink(&self.0).map(__QrdArg_a))
///     }
/// }
/// ```
///
/// Returns `None` for non-overridden args (they pass through their natural
/// type, no newtype needed).
fn newtype_def(plan: &ArgPlan, g_param: &Ident) -> Option<TokenStream2> {
  let path = plan.override_path.as_ref()?;
  let ty = &plan.ty;
  let nt_name = newtype_ident(&plan.ident);

  // We use `::std::boxed::Box` unconditionally here because the generated
  // *test* fn runs under `cfg(test)` where `std` is always available — the
  // crate's `alloc`-only mode applies to the *derive*'s output (which lands
  // in `#![no_std]` consumers), not to this test runner output.
  Some(quote! {
    #[derive(::core::clone::Clone)]
    #[allow(non_camel_case_types)]
    struct #nt_name(#ty);

    impl ::core::fmt::Debug for #nt_name {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Debug::fmt(&self.0, f)
      }
    }

    impl ::quickcheck::Arbitrary for #nt_name {
      fn arbitrary(#g_param: &mut ::quickcheck::Gen) -> Self {
        Self(#path(#g_param))
      }
      fn shrink(&self) -> ::std::boxed::Box<dyn ::core::iter::Iterator<Item = Self>> {
        ::std::boxed::Box::new(
          <#ty as ::quickcheck::Arbitrary>::shrink(&self.0).map(#nt_name)
        )
      }
    }
  })
}

/// Per-override newtype name: `__QrdArg_<ident>`. Embedding the original
/// ident keeps compiler diagnostics readable when the user's generator path
/// has the wrong signature.
fn newtype_ident(arg: &Ident) -> Ident {
  format_ident!("__QrdArg_{}", arg, span = Span::mixed_site())
}

/// Wrapper-fn parameter name for `arg`. We rename to `__qrd_arg_<ident>` so
/// the inner-call expression can rebind the user's actual ident inside the
/// wrapper body without shadowing.
fn wrapper_param_ident(arg: &Ident) -> Ident {
  format_ident!("__qrd_arg_{}", arg, span = Span::mixed_site())
}

/// Wrapper-fn parameter type: the newtype if overridden, else the natural
/// type.
fn wrapper_param_type(plan: &ArgPlan) -> TokenStream2 {
  if plan.override_path.is_some() {
    let nt = newtype_ident(&plan.ident);
    quote!(#nt)
  } else {
    let ty = &plan.ty;
    quote!(#ty)
  }
}
