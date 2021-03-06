use std::fmt;
use crate::func::Func;
use std::ops::Deref;
use std::collections::hash_map::{HashMap, DefaultHasher, Entry};
use std::rc::{Rc, Weak};
use crate::poly::Poly;
use std::hash::{Hash, Hasher};

pub struct Cache {
    items: HashMap<u64, Weak<(Node, u64)>>
}
impl Cache {
    pub fn new() -> Cache {
        Cache { items: HashMap::new() }
    }
    pub fn intern(&mut self, node: Node) -> NodeRc {
        let mut h = DefaultHasher::new();
        node.hash(&mut h);
        let hash = h.finish();
        let rc = match self.items.entry(hash) {
            Entry::Vacant(v) => {
                let rc = Rc::new((node, hash));
                v.insert(Rc::downgrade(&rc));
                rc
            }
            Entry::Occupied(mut o) => {
                match o.get().upgrade() {
                    Some(rc) => {
                        assert_eq!(rc.0, node);
                        rc
                    },
                    None => {
                        let rc = Rc::new((node, hash));
                        o.insert(Rc::downgrade(&rc));
                        rc
                    }
                }
            }
        };
        
        NodeRc { inner: rc }
    }
}
#[derive(Clone, Debug, Ord, PartialOrd)]
pub struct NodeRc {
    inner: Rc<(Node, u64)>,
}
impl Deref for NodeRc {
    type Target = Node;
    fn deref(&self) -> &Node { &self.inner.0 }
}
impl PartialEq for NodeRc {
    fn eq(&self, rhs: &NodeRc) -> bool {
        self.inner.1 == rhs.inner.1
    }
}
impl Eq for NodeRc {}
impl Hash for NodeRc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.inner.1);
    }
}

impl fmt::Display for NodeRc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.0.fmt(f)
    }
}


#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Sign {
    Negative,
    Positive
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Op {
    Diff(String),
}
impl fmt::Display for Op {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Op::Diff(ref v) => write!(w, "d/d{}", v)
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Node {
    Var(String),
    Op(Func),
    Apply(NodeRc, NodeRc),
    Poly(Poly),
    Tuple(Vec<NodeRc>)
}

impl fmt::Display for Node {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        use crate::display::*;
        Tokens::node(self, &Mode::Text).fmt(w)
    }
}
