use quickcheck_richderive::test;

fn my_gen(_g: &mut quickcheck::Gen) -> i32 {
  0
}

#[test(z = "my_gen")]
fn t(a: i32, b: String) -> bool {
  let _ = (a, b);
  true
}

fn main() {}
