use quickcheck_richderive::Arbitrary;

#[derive(Arbitrary)]
#[quickcheck(with = "m", arbitrary = "f")]
struct S {
  x: u8,
}

fn main() {}
