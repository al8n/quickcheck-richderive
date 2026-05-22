//! Struct-shape coverage: named, tuple, unit, `default` and `with`/`shrink`
//! field attributes.

use quickcheck::{Arbitrary, Gen};
use quickcheck_richderive::Arbitrary as DeriveArbitrary;

fn generate() -> Gen {
  Gen::new(16)
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct Named {
  a: u8,
  b: bool,
  c: Vec<i16>,
}

#[test]
fn named_struct_arbitrary_and_shrink() {
  let mut g = generate();
  let value = Named::arbitrary(&mut g);
  // shrink yields an iterator (possibly empty if all fields are minimal).
  let shrinks: Vec<Named> = value.shrink().collect();
  // Every produced shrink differs from the original in some field.
  for s in &shrinks {
    assert_ne!(s, &value);
  }
}

#[test]
fn named_struct_shrink_changes_one_field_at_a_time() {
  let value = Named {
    a: 5,
    b: true,
    c: vec![1, 2],
  };
  let shrinks: Vec<Named> = value.shrink().collect();
  // Each shrink should hold all-but-one field constant.
  for s in &shrinks {
    let diffs = (s.a != value.a) as u32 + (s.b != value.b) as u32 + (s.c != value.c) as u32;
    assert_eq!(diffs, 1, "exactly one field shrinks at a time: {s:?}");
  }
  // There is at least one smaller candidate for a non-trivial value.
  assert!(!shrinks.is_empty());
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct Tup(u16, String);

#[test]
fn tuple_struct() {
  let mut g = generate();
  let value = Tup::arbitrary(&mut g);
  let _shrinks: Vec<Tup> = value.shrink().collect();
  let probe = Tup(3, "hi".into());
  for s in probe.shrink() {
    assert_ne!(s, probe);
  }
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct UnitStruct;

#[test]
fn unit_struct() {
  let mut g = generate();
  let value = UnitStruct::arbitrary(&mut g);
  assert_eq!(value, UnitStruct);
  // Unit struct has nothing to shrink.
  assert_eq!(value.shrink().count(), 0);
}

// --- field `default` ---

#[derive(Clone, Debug, PartialEq, Default)]
struct NoArb(u64);

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct WithDefault {
  a: u8,
  #[quickcheck(default)]
  held: NoArb,
}

#[test]
fn field_default_uses_default_and_is_held_constant() {
  let mut g = generate();
  // Compiles even though `NoArb` is not `Arbitrary` — proves `default` is used.
  let value = WithDefault::arbitrary(&mut g);
  assert_eq!(value.held, NoArb::default());

  let probe = WithDefault {
    a: 9,
    held: NoArb(123),
  };
  // `held` is never shrunk; only `a` may shrink.
  for s in probe.shrink() {
    assert_eq!(s.held, probe.held);
  }
}

// --- field `with` + `shrink` ---

fn make_seven(_g: &mut Gen) -> u32 {
  7
}

fn shrink_to_zero(value: &u32) -> Box<dyn Iterator<Item = u32>> {
  if *value == 0 {
    Box::new(std::iter::empty())
  } else {
    Box::new(std::iter::once(0))
  }
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct CustomField {
  #[quickcheck(arbitrary = "make_seven", shrink = "shrink_to_zero")]
  x: u32,
  y: u8,
}

#[test]
fn field_with_and_shrink_are_used() {
  let mut g = generate();
  let value = CustomField::arbitrary(&mut g);
  assert_eq!(value.x, 7, "field `with` fn must drive generation");

  let probe = CustomField { x: 7, y: 4 };
  let x_shrinks: Vec<u32> = probe
    .shrink()
    .filter(|s| s.y == probe.y)
    .map(|s| s.x)
    .collect();
  assert!(
    x_shrinks.contains(&0),
    "custom `shrink` fn must produce 0 for x"
  );
}

// --- field `with` without `shrink` is held constant ---

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct WithNoShrink {
  #[quickcheck(arbitrary = "make_seven")]
  x: u32,
  y: u8,
}

#[test]
fn field_with_without_shrink_is_held_constant() {
  let probe = WithNoShrink { x: 7, y: 200 };
  for s in probe.shrink() {
    assert_eq!(s.x, probe.x, "x must be held when `with` has no `shrink`");
  }
}
