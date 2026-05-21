use quickcheck_derive::Arbitrary;

// Container `with` would normally skip per-field attribute parsing, but
// `validate_all_attrs` rejects the unknown field attribute anyway.
#[derive(Arbitrary)]
#[quickcheck(with = "f")]
struct S {
  #[quickcheck(bogus)]
  x: u8,
}

fn main() {}
