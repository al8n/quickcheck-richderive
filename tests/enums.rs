//! Enum coverage: unit/tuple/struct variants, `skip`, variant `with`/`shrink`.

use quickcheck::{Arbitrary, Gen};
use quickcheck_richderive::Arbitrary as DeriveArbitrary;

fn generate() -> Gen {
  Gen::new(16)
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum Shape {
  Unit,
  Tuple(u8, bool),
  Struct {
    width: u16,
    height: u16,
  },
  #[quickcheck(skip)]
  Never(String),
}

#[test]
fn enum_skips_marked_variant() {
  let mut g = generate();
  for _ in 0..1000 {
    let value = Shape::arbitrary(&mut g);
    assert!(
      !matches!(value, Shape::Never(_)),
      "`skip` variant must never be generated"
    );
  }
}

#[test]
fn enum_generates_all_non_skipped_variants() {
  let mut g = generate();
  let mut saw_unit = false;
  let mut saw_tuple = false;
  let mut saw_struct = false;
  for _ in 0..2000 {
    match Shape::arbitrary(&mut g) {
      Shape::Unit => saw_unit = true,
      Shape::Tuple(..) => saw_tuple = true,
      Shape::Struct { .. } => saw_struct = true,
      Shape::Never(_) => unreachable!(),
    }
  }
  assert!(saw_unit && saw_tuple && saw_struct);
}

#[test]
fn enum_unit_variant_shrinks_to_empty() {
  let value = Shape::Unit;
  assert_eq!(value.shrink().count(), 0);
}

#[test]
fn enum_tuple_variant_shrinks_one_field_at_a_time() {
  let value = Shape::Tuple(5, true);
  let shrinks: Vec<Shape> = value.shrink().collect();
  for s in &shrinks {
    match s {
      Shape::Tuple(a, b) => {
        // Exactly one of the two fields changed (b can only shrink true->false).
        let changed = (*a != 5) as u32 + (!*b) as u32;
        assert_eq!(changed, 1, "one field at a time: {s:?}");
      }
      other => panic!("shrink changed variant: {other:?}"),
    }
  }
  assert!(!shrinks.is_empty());
}

#[test]
fn enum_struct_variant_shrinks_one_field_at_a_time() {
  let value = Shape::Struct {
    width: 10,
    height: 20,
  };
  for s in value.shrink() {
    match s {
      Shape::Struct { width, height } => {
        let changed = (width != 10) as u32 + (height != 20) as u32;
        assert_eq!(changed, 1);
      }
      other => panic!("shrink changed variant: {other:?}"),
    }
  }
}

#[test]
fn enum_skip_value_shrinks_to_empty() {
  // A value that *is* the skipped variant still shrinks to empty.
  let value = Shape::Never("hi".into());
  assert_eq!(value.shrink().count(), 0);
}

// --- variant-level `with` / `shrink` ---

fn make_special(_g: &mut Gen) -> Decorated {
  Decorated::Special(42)
}

fn shrink_special(value: &Decorated) -> Box<dyn Iterator<Item = Decorated>> {
  match value {
    Decorated::Special(n) if *n > 0 => Box::new(std::iter::once(Decorated::Special(0))),
    _ => Box::new(std::iter::empty()),
  }
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum Decorated {
  Plain(u8),
  #[quickcheck(arbitrary = "make_special", shrink = "shrink_special")]
  Special(u32),
}

#[test]
fn variant_with_drives_generation_and_shrink() {
  let mut g = generate();
  let mut saw_special_42 = false;
  for _ in 0..500 {
    if let Decorated::Special(n) = Decorated::arbitrary(&mut g) {
      assert_eq!(n, 42, "variant `with` fn must build the value");
      saw_special_42 = true;
    }
  }
  assert!(saw_special_42);

  let value = Decorated::Special(99);
  let shrinks: Vec<Decorated> = value.shrink().collect();
  assert_eq!(shrinks, vec![Decorated::Special(0)]);
}

// --- variant `with` precedence over field attrs + `with`-without-`shrink` ---

fn build_combo(_g: &mut Gen) -> Combo {
  Combo::Made { x: 1, y: 2 }
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum Combo {
  // Variant `with` present, no variant `shrink` => generation via fn, shrink empty.
  #[quickcheck(arbitrary = "build_combo")]
  Made {
    x: u8,
    // Field attrs here must be ignored because variant `with` takes precedence.
    #[quickcheck(default)]
    y: u8,
  },
  Other(u8),
}

#[test]
fn variant_with_precedence_and_empty_shrink() {
  let mut g = generate();
  let mut saw_made = false;
  for _ in 0..500 {
    match Combo::arbitrary(&mut g) {
      Combo::Made { x, y } => {
        assert_eq!((x, y), (1, 2), "variant `with` overrides field attrs");
        saw_made = true;
      }
      Combo::Other(_) => {}
    }
  }
  assert!(saw_made);

  // `with` without `shrink` => empty shrink for that variant.
  let value = Combo::Made { x: 1, y: 2 };
  assert_eq!(value.shrink().count(), 0);
}

// Manual-`Clone` enum with a non-`Clone` `#[quickcheck(default)]` held field
// alongside a shrinkable field (round-7 finding): enum shrink must clone the
// whole `Self` (relying on `Self: Clone`) and never clone the held field, so
// `NotCloneButDefault` need not be `Clone`.
#[derive(Default)]
struct NotCloneButDefault;

#[allow(dead_code)] // `held` is generated/cloned but never read in the test.
#[derive(DeriveArbitrary)]
enum HeldEnum {
  V {
    keep: u8,
    #[quickcheck(default)]
    held: NotCloneButDefault,
  },
  W,
}

impl Clone for HeldEnum {
  fn clone(&self) -> Self {
    match self {
      Self::V { keep, .. } => Self::V {
        keep: *keep,
        held: NotCloneButDefault,
      },
      Self::W => Self::W,
    }
  }
}

#[test]
fn manual_clone_enum_with_non_clone_held_field() {
  let value = HeldEnum::V {
    keep: 9,
    held: NotCloneButDefault,
  };
  // Shrinking `keep` must not require `NotCloneButDefault: Clone`.
  let _ = value.shrink().count();
  let mut g = generate();
  let _ = HeldEnum::arbitrary(&mut g);
}

// Single-variant enums: the in-place slot assignment must not emit an
// irrefutable-`if let` warning (round-8). This file compiles under `-D warnings`.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum SingleNamed {
  Only { a: u8, b: u16 },
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum SingleTuple {
  Only(u8, u16),
}

#[test]
fn single_variant_enum_shrink() {
  let n = SingleNamed::Only { a: 5, b: 9 };
  for s in n.shrink() {
    match s {
      SingleNamed::Only { a, b } => assert!(a != 5 || b != 9),
    }
  }
  let t = SingleTuple::Only(5, 9);
  let _ = t.shrink().count();
  let mut g = generate();
  let _ = SingleNamed::arbitrary(&mut g);
  let _ = SingleTuple::arbitrary(&mut g);
}
