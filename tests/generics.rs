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

// --- projected / associated-type field inference (round-2 finding A) ---
//
// The generated field is `T::Item`, not `T`. The derive must bound the projected
// *type* (`<T as Carrier>::Item: Arbitrary`), NOT `T: Arbitrary` — the latter
// would not imply the projection is `Arbitrary`, and would wrongly demand
// `T: Arbitrary`. `NotArb` is deliberately NOT `Arbitrary` but its `Item` (u32)
// is, so this compiles only with field-type-based inference.
trait Carrier {
  type Item;
}

impl Carrier for NotArb {
  type Item = u32;
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct Projected<T: Carrier>
where
  T: Clone + 'static,
  <T as Carrier>::Item: Clone + core::fmt::Debug + PartialEq,
{
  item: T::Item,
  tag: u8,
}

#[test]
fn projected_field_type_is_bounded() {
  let mut g = gen();
  // Compiles only because the derive bounds `<T as Carrier>::Item: Arbitrary`
  // (here `u32`), NOT `T: Arbitrary` — `NotArb` is not `Arbitrary`.
  let value: Projected<NotArb> = Projected::arbitrary(&mut g);
  let _shrinks: Vec<Projected<NotArb>> = value.shrink().collect();
}

// --- const-generic field-type inference (round-3 finding A) ---
//
// `Only<N>` is `Arbitrary` only for `N == 3`. The generated field `inner:
// Only<N>` must yield a `where Only<N>: Arbitrary` predicate (a const param
// counts as "mentioning a generic param"); otherwise the generic body would
// require `Only<N>: Arbitrary` for all `N` and fail to compile.
#[allow(non_upper_case_globals)]
#[derive(Clone, Debug, PartialEq)]
struct Only<const N: usize>;

impl Arbitrary for Only<3> {
  fn arbitrary(_g: &mut Gen) -> Self {
    Only
  }
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct ConstField<const N: usize> {
  inner: Only<N>,
  tag: u8,
}

#[test]
fn const_generic_field_type_is_bounded() {
  let mut g = gen();
  // Compiles only because the derive emits `where Only<N>: Arbitrary`
  // (`Only<3>: Arbitrary` holds).
  let value: ConstField<3> = ConstField::arbitrary(&mut g);
  let _shrinks: Vec<ConstField<3>> = value.shrink().collect();
}

// --- nested generic field types satisfy the `Clone` supertrait (round-5) ---
//
// `Vec<T>` / `Option<T>` fields get a `<FieldTy>: Arbitrary` bound; every type
// param additionally gets `T: Clone + 'static`, so the `Arbitrary: Clone +
// 'static` supertrait on `Self` is satisfiable. (`Vec<T>: Arbitrary` alone does
// not imply `T: Clone`, which the derived `Clone for Nested<T>` requires.)
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct Nested<T> {
  xs: Vec<T>,
  maybe: Option<T>,
}

#[test]
fn nested_generic_fields_compile() {
  let mut g = gen();
  let value: Nested<u8> = Nested::arbitrary(&mut g);
  let _shrinks: Vec<Nested<u8>> = value.shrink().collect();
}
