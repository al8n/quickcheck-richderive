//! Container `box = "..."` override for the `shrink` return `Box` type.
//!
//! The `std` / `alloc` features pick the default (`::std::boxed::Box` /
//! `::alloc::boxed::Box`); `box = "..."` overrides either. We can't exercise the
//! no-std default from this std test crate, but the override drives the exact
//! same box-path substitution end to end.

use quickcheck::{Arbitrary, Gen};
use quickcheck_derive::Arbitrary as DeriveArbitrary;

// A custom path to a `Box` type (here, a re-export of the std one).
mod custom {
  pub use std::boxed::Box;
}

#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
#[quickcheck(box = "custom::Box")]
struct WithBox {
  x: u32,
  y: u8,
}

#[test]
fn box_override_struct() {
  let mut g = Gen::new(16);
  let value: WithBox = WithBox::arbitrary(&mut g);
  // `shrink` returns `custom::Box<dyn Iterator<…>>` (== std `Box`).
  let shrinks: custom::Box<dyn Iterator<Item = WithBox>> = value.shrink();
  let _ = shrinks.count();
}

// Enum override exercises `variant_shrink` + the empty-iterator paths with the
// custom box.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
#[quickcheck(box = "custom::Box")]
enum WithBoxEnum {
  A(u8, u16),
  B,
}

#[test]
fn box_override_enum() {
  let mut g = Gen::new(16);
  let _value = WithBoxEnum::arbitrary(&mut g);
  let _ = WithBoxEnum::A(1, 2).shrink().count();
  let _ = WithBoxEnum::B.shrink().count();
}
