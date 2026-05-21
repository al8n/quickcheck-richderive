//! Generic structs: inferred bounds and explicit `bound = "..."`.

use quickcheck::{Arbitrary, Gen};
use quickcheck_derive::Arbitrary as DeriveArbitrary;

fn gen() -> Gen {
  Gen::new(16)
}

// Inferred bound: `T: ::quickcheck::Arbitrary`.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct Wrapper<T> {
  inner: T,
  tag: u8,
}

#[test]
fn generic_inferred_bound() {
  let mut g = gen();
  let value: Wrapper<Vec<u8>> = Wrapper::arbitrary(&mut g);
  let probe = Wrapper {
    inner: value.inner.clone(),
    tag: value.tag,
  };
  let _shrinks: Vec<Wrapper<Vec<u8>>> = probe.shrink().collect();
}

// Explicit (odd) bound combined with a `default` field so the type need not
// itself be `Arbitrary` — proves `bound` replaces the inferred bound.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
#[quickcheck(bound = "T: Clone + Default + 'static")]
struct Holder<T> {
  #[quickcheck(default)]
  inner: T,
  n: u16,
}

#[derive(Clone, Debug, PartialEq, Default)]
struct NotArbitrary(String);

#[test]
fn explicit_bound_with_default_field() {
  let mut g = gen();
  // `NotArbitrary` is NOT `Arbitrary`; compiles only because the bound is
  // `T: Clone + Default` and the field uses `default`.
  let value: Holder<NotArbitrary> = Holder::arbitrary(&mut g);
  assert_eq!(value.inner, NotArbitrary::default());
  let _shrinks: Vec<Holder<NotArbitrary>> = value.shrink().collect();
}

// Two generic params, both inferred.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct Pair<A, B> {
  a: A,
  b: B,
}

#[test]
fn two_param_generic() {
  let mut g = gen();
  let value: Pair<u8, bool> = Pair::arbitrary(&mut g);
  let _ = value.shrink().count();
}

// --- usage-based bound inference (Finding #1) ---

// A concrete type that is deliberately NOT `Arbitrary`, used to prove that
// inference does not require `T: Arbitrary` when `T` is never generated via
// `Arbitrary::arbitrary`.
#[derive(Clone, Debug, PartialEq, Default)]
struct NotArb;

// A plain generic wrapper still infers `T: Arbitrary` (existing behavior).
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct W<T>(T);

#[test]
fn plain_generic_still_infers_arbitrary() {
  let mut g = gen();
  let value: W<u8> = W::arbitrary(&mut g);
  let _ = value.shrink().count();
}

// `T` only appears in a `#[quickcheck(default)]` field, so no `T: Arbitrary`
// bound is inferred; `WithDefault::<NotArb>` must still be `Arbitrary`.
//
// The struct's own `where T: Clone + Default + 'static` covers what the derive
// needs to clone/box `Self` — crucially WITHOUT `T: Arbitrary`, which is the
// point: a non-`Arbitrary` `T` (here `NotArb`) compiles because inference no
// longer demands `T: Arbitrary` for a `default` field.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct WithDefault<T>
where
  T: Clone + Default + 'static,
{
  #[quickcheck(default)]
  inner: T,
  x: u32,
}

#[test]
fn default_field_param_not_bounded() {
  let mut g = gen();
  // Compiles only if no `T: Arbitrary` bound was inferred (`NotArb` is not Arbitrary).
  let value: WithDefault<NotArb> = WithDefault::arbitrary(&mut g);
  assert_eq!(value.inner, NotArb);
  let _shrinks: Vec<WithDefault<NotArb>> = value.shrink().collect();
}

// `T` only appears in a `#[quickcheck(skip)]` variant, so no `T: Arbitrary`
// bound is inferred; `E::<NotArb>` must still be `Arbitrary`. The struct's own
// `where T: Clone + 'static` covers cloning/boxing without requiring Arbitrary.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum E<T>
where
  T: Clone + 'static,
{
  // Never generated (skip) and never constructed in the test; we only assert it
  // does not force `T: Arbitrary`.
  #[allow(dead_code)]
  #[quickcheck(skip)]
  A(T),
  B(u8),
}

#[test]
fn skip_variant_param_not_bounded() {
  let mut g = gen();
  // Compiles only if no `T: Arbitrary` bound was inferred for the skipped variant.
  let value: E<NotArb> = E::arbitrary(&mut g);
  assert!(matches!(value, E::B(_)));
  let _shrinks: Vec<E<NotArb>> = value.shrink().collect();
}

// Concrete field-level `with`: `T` is bound by the fn's return type, never via
// `Arbitrary`, so no `T: Arbitrary` bound is inferred. The `with` fn produces
// the concrete `NotArb`, so this type is only ever `NotConcrete` (no generic
// inference of `T` from the fn). Using a concrete monomorphic alias keeps it
// well-formed while still exercising the field-`with` no-bound path.
fn make_notarb(_g: &mut Gen) -> NotArb {
  NotArb
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct FieldWith {
  #[quickcheck(with = "make_notarb")]
  inner: NotArb,
  x: u8,
}

#[test]
fn field_with_no_extra_bound() {
  let mut g = gen();
  // Compiles even though `NotArb` is not `Arbitrary`: the field uses `with`.
  let value = FieldWith::arbitrary(&mut g);
  assert_eq!(value.inner, NotArb);
  let _shrinks: Vec<FieldWith> = value.shrink().collect();
}
