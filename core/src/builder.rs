use prelude::*;
use std::cell::RefCell;
use func::Func;
use func::Transient::*;
use poly::Poly;
use lang::parse_Expr;
use std::collections::HashMap;
use std::iter::once;

pub type NodeResult = Result<NodeRc, Error>;

struct Definition {
    args: Vec<String>,
    expr: NodeRc
}

pub struct Builder {
    cache: RefCell<Cache>,
    defs: HashMap<String, Definition>
}

fn poly(node: NodeRc) -> Poly {
    if let Node::Poly(ref p) = *node {
        return p.clone();
    }
    Poly::from_node(node)
}

impl Builder {
    pub fn new() -> Builder {
        let mut b = Builder {
            cache: RefCell::new(Cache::new()),
            defs:  HashMap::new()
        };
        b.init();
        b
    }
    fn init(&mut self) {
        let x = self.var("x");
        for &(n, f) in [("sin", Sin), ("cos", Cos), ("exp", Exp), ("log", Log)].iter() {
            let f = self.func(Func::Transient(f), x.clone()).unwrap();
            self.define(n, &["x"], f);
        }
    }
    pub fn define(&mut self, name: &str, args: &[&str], node: NodeRc) {
        self.defs.insert(name.to_owned(), Definition {
            args: args.iter().map(|&s| s.into()).collect(),
            expr: node
        });
    }
    pub fn parse(&self, expr: &str) -> NodeResult {
        parse_Expr(self, expr).unwrap_or_else(|e| Err(Error::parse_error(e, expr)))
    }
    pub fn int(&self, i: i64) -> NodeRc {
        self.intern(Node::Poly(Poly::int(i)))
    }
    
    /// decimal number
    pub fn decimal(&self, n: &str) -> NodeResult {
        let i: i64 = n.parse().map_err(|_| Error::IntegerError)?;
        Ok(self.int(i))
    }
    pub fn decimal_float(&self, s: &str) -> NodeResult {
        let dp = s.find('.').unwrap();
        let div = 10i64.pow((s.len() - dp - 1) as u32);
        let i: i64 = s[..dp].parse().map_err(|_| Error::IntegerError)?;
        let j: i64 = s[dp+1..].parse().map_err(|_| Error::IntegerError)?;
        self.add(self.int(i), self.div(self.int(j), self.int(div))?)
    }

    pub fn poly(&self, p: Poly) -> NodeRc {
        self.intern(Node::Poly(p))
    }

    fn uniform<F>(&self, a: NodeRc, b: NodeRc, f: F) -> NodeResult
        where F: Fn(NodeRc, NodeRc) -> NodeResult
    {
        match (&*a, &*b) {
            (&Node::Tuple(ref ta), &Node::Tuple(ref tb)) => {
                if ta.len() != tb.len() {
                    return Err(Error::ShapeMismatch(ta.len(), tb.len()));
                }
                self.tuple(
                    ta.iter().zip(tb.iter()).map(|(a, b)| f(a.clone(), b.clone()))
                )
            },
            (&Node::Tuple(ref ta), _) => self.tuple(ta.iter().map(|a| f(a.clone(), b.clone()))),
            (_, &Node::Tuple(ref tb)) => self.tuple(tb.iter().map(|b| f(a.clone(), b.clone()))),
            (_, _) => f(a.clone(), b.clone())
        }
    }
    fn uniform_one<F, T>(&self, a: NodeRc, t: T, f: F) -> NodeResult
        where F: Fn(NodeRc, T) -> NodeResult, T: Clone
    {
        match *a {
            Node::Tuple(ref ta) => self.tuple(ta.iter().map(|a| f(a.clone(), t.clone()))),
            _ => f(a.clone(), t)
        }
    }
    /// a + b
    pub fn add(&self, a: NodeRc, b: NodeRc) -> NodeResult {
        self.uniform(a, b, |a, b| Ok(self.poly(poly(a) + poly(b))))
    }

    /// a - b
    pub fn sub(&self, a: NodeRc, b: NodeRc) -> NodeResult {
        self.uniform(a, b, |a, b| Ok(self.poly(poly(a) + poly(b) * (-1))))
    }

    /// a * b
    pub fn mul(&self, a: NodeRc, b: NodeRc) -> NodeResult {
        self.uniform(a, b, |a, b| Ok(self.poly(poly(a) * poly(b))))
    }

    /// a / b
    pub fn div(&self, a: NodeRc, b: NodeRc) -> NodeResult {
        self.uniform(a, b, |a, b| Ok(self.poly(poly(a) * poly(b).pow_i(self, -1)?)))
    }

    /// - a
    pub fn neg(&self, a: NodeRc) -> NodeResult {
        self.mul(self.int(-1), a)
    }
    
    /// a ^ b
    pub fn pow(&self, a: NodeRc, b: NodeRc) -> NodeResult {
        self.uniform(a, b, |a, b| {
            if let Node::Poly(ref p) = *b {
                if let Some(i) = p.as_int().and_then(|i| i.cast()) {          
                    return Ok(self.pow_i(a, i)?);
                }
            }
            
            let g = self.func(Log.into(), a)?;
            self.func(Exp.into(), self.mul(g, b)?)
        })
    }
    /// a ^ i
    pub fn pow_i(&self, a: NodeRc, i: i32) -> NodeResult {
        self.uniform_one(a, i, |a, i| Ok(self.poly(poly(a).pow_i(self, i)?)))
    }

    /// f(g)
    pub fn func(&self, f: Func, g: NodeRc) -> NodeResult {
        self.uniform_one(g, f, |g, f| Ok(self.intern(Node::Func(f, g))))
    }

    /// make a name variable
    pub fn var(&self, name: &str) -> NodeRc {
        self.intern(Node::Var(name.into()))
    }

    /// magic 'apply' function
    pub fn apply(&self, left: NodeRc, right: NodeRc) -> NodeResult {
        match *left {
            Node::Var(ref name) => {
                if let Some(def) = self.defs.get(name) {
                    let map = |args: &[NodeRc]| -> HashMap<&str, NodeRc> {
                        args.iter()
                            .zip(def.args.iter())
                            .map(|(subst, var)| (&**var, subst.clone()))
                            .collect()
                    };

                    return match *right {
                        Node::Tuple(ref parts) => match def.args.len() {
                            1 => {
                                self.tuple(parts.windows(1).map(|p| self.substitute(&def.expr, &map(p))))
                            },
                            n if n == parts.len() => self.substitute(&def.expr, &map(parts)),
                            n => Err(Error::ShapeMismatch(n, parts.len()))
                        },
                        _ if def.args.len() == 1 => self.substitute(&def.expr, &map(&[right.clone()])),
                        _ => Err(Error::ShapeMismatch(def.args.len(), 1))
                    };
                }
            },
            Node::Poly(ref _p) => {
                
            },
            _ => {}
        }
        self.mul(left, right)
    }

    fn substitute(&self, node: &NodeRc, map: &HashMap<&str, NodeRc>) -> NodeResult {
        match **node {
            Node::Var(ref name) => match map.get(&**name) {
                Some(node) => Ok(node.clone()),
                None => Ok(node.clone())
            },
            Node::Tuple(ref parts) => self.tuple(parts.iter().map(|n| self.substitute(n, map))),
            Node::Poly(ref p) => self.sum(
                p.factors().map(|(base, &fac)| {
                    self.product(
                        once(Ok(self.rational(fac)))
                            .chain(
                                base.iter().map(|&(ref v, p)| self.pow_i(
                                    self.substitute(v, map)?,
                                    p.cast().expect("too high")
                                ))
                            )
                    )
                })
            ),
            Node::Func(ref f, ref n) => self.func(f.clone(), self.substitute(n, map)?)
        }
    }

    /// f_0 · f_1 · f_2 · … · f_n
    pub fn product<I>(&self, factors: I) -> NodeResult
        where I: IntoIterator<Item=NodeResult>
    {
        try_fold(factors, self.int(1), |a, b| self.mul(a, b))
    }

    /// f_0 + f_1 + f_2 + … + f_n
    pub fn sum<I>(&self, summands: I) -> NodeResult
        where I: IntoIterator<Item=NodeResult>
    {
        try_fold(summands, self.int(0), |a, b| self.add(a, b))
    }

    pub fn rational(&self, r: Rational) -> NodeRc {
        self.poly(Poly::rational(r))
    }

    pub fn tuple<I>(&self, parts: I) -> NodeResult
        where I: IntoIterator<Item=NodeResult>
    {
        let v: Result<Vec<_>, _> = parts.into_iter().collect();
        Ok(self.intern(Node::Tuple(v?)))
    }

    pub fn array<I>(&self, _parts: I) -> NodeResult
        where I: IntoIterator<Item=NodeResult>
    {
        //let v: Result<Vec<_>> = parts.into_iter().collect();
        todo!("arrays")
    }
    
    pub fn intern(&self, node: Node) -> NodeRc {
        self.cache.borrow_mut().intern(node).clone()
    }
}
