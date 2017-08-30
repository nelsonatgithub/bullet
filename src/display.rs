use node::Node;
use poly::Poly;
use itertools::Itertools;
use std::fmt::{self, Display};

fn int_super(i: i64) -> String {
    i.to_string().chars().map(|c| {
        match c {
            '-' => '⁻',
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            _ => unreachable!()
        }
    }).collect()
}

pub struct Tokens {
    content: Vec<String>
}
impl Display for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut iter = self.content.iter();
        if let Some(first) = iter.next() {
            f.write_str(&first)?;
            for s in iter {
                f.write_str(" ")?;
                f.write_str(&s)?;
            }
        }
        Ok(())
    }
}
fn wrap_poly(p: &Poly) -> String {
    let mut tokens = Tokens::new();
    tokens.poly(p);
    if tokens.len() > 1 {
        format!("({})", tokens)
    } else {
        tokens.to_string()
    }
}

impl Tokens {
    pub fn new() -> Tokens {
        Tokens { content: vec![] }
    }
    pub fn len(&self) -> usize {
        self.content.len()
    }
    pub fn push<T: fmt::Display>(&mut self, t: T) {
        self.content.push(t.to_string());
    }
    pub fn poly(&mut self, p: &Poly) {
        let elements: Vec<_> = p.factors().collect();

        for (n, &(base, fac)) in elements.iter().enumerate() {
            let (nom, denom) = fac.frac();

            if nom < 0 {
                self.push("-");
            } else if n != 0 {
                self.push("+");
            }
            if nom.abs() != 1 || base.len() == 0 {
                self.push(nom.abs());
            }

            for &(ref v, n) in base.iter() {
                if n == 1 {
                    self.push(v);
                } else {
                    self.push(format!("{}{}", v, int_super(n)));
                }
            }

            match denom {
                1 => {},
                d => {
                    self.push("/");
                    self.push(d);
                }
            }
        }
        if self.len() == 0 {
            self.push("0");
        }
    }
    pub fn node(n: &Node) -> Tokens {
        let mut tokens = Tokens::new();
        match *n {
            Node::Func(f, ref g) => {
                tokens.push(format!("{}({})", f, Tokens::node(g)));
            },
            Node::Poly(ref p) => {
                match p.factorize() {
                    Some((p, q)) => {
                        tokens.push(wrap_poly(&p));
                        tokens.push(wrap_poly(&q));
                    },
                    None => tokens.poly(p),
                }
            }
            Node::Var(ref name) => tokens.push(name),
            Node::Tuple(ref parts) => tokens.push(format!("({})", parts.iter().map(|n| Tokens::node(n)).join(", ")))
        }
        tokens
    }
}