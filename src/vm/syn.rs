use prelude::*;
use compiler::Compiler;
use vm::{Vm, Round};
use quote::{Tokens, Ident};
use std::mem;

struct Syn {
    tokens: Tokens,
    stored: usize,
    inputs: Vec<Ident>
}
impl Syn {
    pub fn new() -> Syn {
        Syn {
            tokens: Tokens::new(),
            stored: 0,
            inputs: Vec::new()
        }
    }
}

impl Vm for Syn {
    type Var = Tokens;
    type Storage = Ident;

    fn make_int(&mut self, i: i64) -> Self::Var {
        let i: i16 = i.cast().unwrap();
        quote! { Real::int(#i) }
    }
    fn make_const(&mut self, x: f64) -> Self::Var {
        quote! { #x }
    }
    fn make_source(&mut self, name: &str) -> Self::Var {
        let name = Ident::from(name);
        self.inputs.push(name.clone());
        quote! { #name }
    }
    fn make_sum(&mut self, parts: Vec<Self::Var>) -> Self::Var {
        let first = &parts[0];
        let others = &parts[1..];
        quote! { #first #(.add(#others) )* }
    }
    fn make_product(&mut self, parts: Vec<Self::Var>) -> Self::Var {
        let first = &parts[0];
        let others = &parts[1..];
        quote! { #first #( .mul( #others ) )* }
    }
    fn store(&mut self, var: &mut Self::Var, _uses: usize) -> Self::Storage {
        // `var` contains an expression.
        // This expression needs to be assigned to a variable, so it can be used again later.

        // make a new variable name
        let name = format!("storage_{}", self.stored).into();
        self.stored += 1;

        // replace `var` by the variable name (load does that) and get ownership of the original expression
        let var = mem::replace(var, self.load(&name));

        // now actually assign the expression to the variable (this is fine, as var can't be used until this function returns)
        self.tokens.append(quote! { let #name = #var; });

        name
    }
    fn load(&mut self, name: &Self::Storage) -> Self::Var {
        quote! { #name }
    }
    fn round(&mut self, x: Self::Var, mode: Round) -> Self::Var {
        match mode {
            Round::Up => quote! { #x.ceil() },
            Round::Down => quote! { #x.floor() }
        }
    }
    fn div(&mut self, a: Self::Var, b: Self::Var) -> Self::Var {
        quote! { #a / #b }
    }
    fn inv(&mut self, a: Self::Var) -> Self::Var {
        quote! { #a.inv() }
    }
    fn step_at(&mut self, at: Self::Var, x: Self::Var) -> Self::Var {
        quote! {
            #x.ge(#at).select(f32x8::splat(1.0), f32::splat(0.0))
        }
    }
    /*
    fn sin(&mut self, x: Self::Var) -> Self::Var {
        quote! { #x.sin() }
    }
    fn cos(&mut self, x: Self::Var) -> Self::Var {
        quote! { #x.cos() }
    }
    */
}

pub fn syn(node: NodeRc) -> Tokens {
    let mut syn = Syn::new();
    let inner = Compiler::run(&mut syn, &node).unwrap();
    let store = syn.tokens;
    let params = syn.inputs.iter().map(|i| quote! { #i: T });
    let args = &syn.inputs;
    quote! {
        {
            extern crate math_traits;
            use math_traits::Real;
            
            #[allow(unused_imports)]
            fn f<T: Real>(#(#params),*) -> T {
                use std::ops::*;
                #store
                #inner
            }
            f(#args)
        }
    }
}
