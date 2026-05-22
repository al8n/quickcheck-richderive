use quickcheck_richderive::Arbitrary;

#[derive(Arbitrary)]
struct S {
  #[quickcheck(with = "m", arbitrary = "f")]
  x: u8,
}

fn main() {}
