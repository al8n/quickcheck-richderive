use quickcheck_richderive::Arbitrary;

#[derive(Arbitrary)]
#[quickcheck(with = "m", shrink = "s")]
struct S {
  x: u8,
}

fn main() {}
