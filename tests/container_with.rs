//! Container-level `with` / `shrink` overrides for the whole type.

use quickcheck::{Arbitrary, Gen};
use quickcheck_richderive::Arbitrary as DeriveArbitrary;

fn generate() -> Gen {
  Gen::new(16)
}

fn build_whole(_g: &mut Gen) -> Whole {
  Whole { a: 100, b: 200 }
}

fn shrink_whole(value: &Whole) -> Box<dyn Iterator<Item = Whole>> {
  if value.a > 0 {
    Box::new(std::iter::once(Whole { a: 0, b: value.b }))
  } else {
    Box::new(std::iter::empty())
  }
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
#[quickcheck(arbitrary = "build_whole", shrink = "shrink_whole")]
struct Whole {
  a: u32,
  b: u32,
}

#[test]
fn container_with_and_shrink() {
  let mut g = generate();
  let value = Whole::arbitrary(&mut g);
  assert_eq!(value, Whole { a: 100, b: 200 });

  let probe = Whole { a: 5, b: 7 };
  let shrinks: Vec<Whole> = probe.shrink().collect();
  assert_eq!(shrinks, vec![Whole { a: 0, b: 7 }]);
}

// Container `with` without `shrink` => generation via fn, shrink empty.
fn build_only(_g: &mut Gen) -> OnlyWith {
  OnlyWith(9)
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
#[quickcheck(arbitrary = "build_only")]
struct OnlyWith(u8);

#[test]
fn container_with_without_shrink_is_empty() {
  let mut g = generate();
  assert_eq!(OnlyWith::arbitrary(&mut g), OnlyWith(9));
  assert_eq!(OnlyWith(9).shrink().count(), 0);
}

// Container `shrink` without `with` => generation is field-derived.
fn shrink_field_derived(value: &FieldGen) -> Box<dyn Iterator<Item = FieldGen>> {
  if value.0 > 0 {
    Box::new(std::iter::once(FieldGen(0)))
  } else {
    Box::new(std::iter::empty())
  }
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
#[quickcheck(shrink = "shrink_field_derived")]
struct FieldGen(u8);

#[test]
fn container_shrink_without_with() {
  let mut g = generate();
  // Generation still works (field-derived), shrink uses the custom fn.
  let _ = FieldGen::arbitrary(&mut g);
  assert_eq!(FieldGen(4).shrink().collect::<Vec<_>>(), vec![FieldGen(0)]);
}
