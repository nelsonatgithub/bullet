extern crate bullet;
use bullet::builder::Builder;

#[test]
fn text_simplify() {
    let pairs = [
        ("1 + 2", "3"),
        ("a / a", "1"),
        ("a ^ b", "exp(b * log(a))"),
        ("d/dx ln x", "1 / x"),
        ("3^3", "27")
    ];
    let builder = Builder::new();
    for &(a, b) in &pairs {
        dbg!((a, b));
        assert_eq!(builder.parse(a).unwrap(), builder.parse(b).unwrap());
    }
}
