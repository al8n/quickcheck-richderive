use quickcheck_derive::Arbitrary;

// Variant `with` would normally skip per-field attribute parsing, but
// `validate_all_attrs` rejects the conflicting field attrs anyway.
#[derive(Arbitrary)]
enum E {
  #[quickcheck(with = "f")]
  V {
    #[quickcheck(default, with = "x")]
    a: u8,
  },
  Other(u8),
}

fn main() {}
