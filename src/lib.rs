#![doc = include_str!("../README.md")]
use std::mem;

use convert_case::{Case, Casing};
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Group, Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
    braced, bracketed, parse::Parse, parse_macro_input, parse_quote, token::Bracket, FnArg, Ident,
    ImplItem, ItemImpl, ItemTrait, Meta, Token, TraitItem, Type, Visibility,
};

/// Replace all mentions of `crate` with `$crate`.
fn decratify(tokens: &mut TokenStream) {
    let mut result = Vec::new();
    for mut tt in mem::take(tokens).into_iter() {
        match &mut tt {
            TokenTree::Group(group) => {
                let span = group.span();
                let mut stream = group.stream();
                decratify(&mut stream);
                *group = Group::new(group.delimiter(), stream);
                group.set_span(span);
            }
            TokenTree::Ident(ident) if ident == "crate" => {
                let mut p = Punct::new('$', Spacing::Alone);
                p.set_span(ident.span());
                result.push(TokenTree::Punct(p));
            }
            _ => (),
        }
        result.push(tt);
    }
    *tokens = result.into_iter().collect()
}

/// Generates a macro that fills missing trait items in an `impl` block by inheriting from one of its fields.
///
/// # Syntax
///
/// This generates macro `impl_my_trait!`.
///
/// ```
/// # use trait_deref::trait_deref;
/// #[trait_deref]
/// pub trait MyTrait { .. }
/// ```
///
/// To rename the macro:
///
/// ```
/// # use trait_deref::trait_deref;
/// #[trait_deref(impl_this)]
/// pub trait MyTrait { .. }
/// ```
///
/// # Use Cases
///
/// This macro is useful on large traits where the user want to use composition
/// to build up features while inheriting most items from the base object.
///
/// # Example
///
/// Imagine we have a card game with a large trait `Card`,
/// this inherits most items from `base` while overwriting `get_cost` and `IS_FIXED_COST`.
///
/// ```
/// # /*
/// struct CardCostExtension<T: Card> {
///     base: T,
///     cost: i32
/// }
///
/// impl_card! {
///     // dereferences to field self.base for missing items.
///     @[base: T]
///     impl<T: Card> Card for CardCostExtension<T> {
///         // overwrites some items.
///         fn get_cost(&self) -> i32 {
///             self.cost
///         }
///
///         const IS_FIXED_COST: bool = true;
///     }
/// }
/// # */
/// ```
///
/// # Rc
///
/// To both erase and compose something like `Rc<dyn MyTrait>`, a special syntax is needed
/// since the type system cannot be infinitely recursive.
///
/// ```
/// # use trait_deref::trait_deref;
/// #[trait_deref]
/// pub trait MyTrait {
///     #[rc]
///     fn get<RC: Clone>(this: RC, get: impl Fn(&RC) -> &Self, ..) -> i32;
///
///     // Since this trait cannot be dyn compatible, you might want to put this in a separate trait.
///     fn get_arc(self: Arc<Self>, ..) -> i32{
///         self.get(self, Arc::as_ref, ..)
///     }
///     
/// }
/// ```
///
/// The trait bound on `RC` can be tailored to your specific needs, for instance `Into<Arc<dyn ErasedMyTrait>>`.
///
/// # import
///
/// The macro cannot find the path of items automatically, so add them manually with `#[import]`:
///
/// ```
/// # use trait_deref::trait_deref;
/// #[trait_deref]
/// #[import(::std::sync::Arc)]
/// // `crate` means `$crate` here since `$crate` is an unstable feature and not allowed.
/// #[import(crate::Card)]
/// pub trait Deck {
///     fn get(self, name: Arc<str>) -> Card;
/// }
/// ```
///
/// # Rules
///
/// * The macro does not rewrite the trait, except removing attributes specific to this macro.
/// * Default function or const implementations will not be used.
/// * Receivers like `self: Box<Self>` is not supported and such items will be ignored.
#[proc_macro_attribute]
pub fn trait_deref(args: TokenStream1, trait_block: TokenStream1) -> TokenStream1 {
    let mut item_trait = parse_macro_input!(trait_block as ItemTrait);

    let mut trait_out = item_trait.clone();
    let mut imports = Vec::new();
    trait_out.attrs.retain(|x| {
        if x.path().is_ident("import") {
            match &x.meta {
                Meta::List(list) => {
                    let mut tokens = list.tokens.clone();
                    decratify(&mut tokens);
                    imports.push(tokens);
                    false
                }
                _ => true,
            }
        } else {
            true
        }
    });
    for item in &mut trait_out.items {
        if let TraitItem::Fn(f) = item {
            f.attrs.retain_mut(|x| !x.path().is_ident("rc"));
        }
    }

    let ident = item_trait.ident.clone();

    let name = if let Ok(name) = syn::parse::<Ident>(args) {
        name
    } else {
        let ident = ident.to_string().to_case(Case::Snake);
        format_ident!("impl_{ident}")
    };

    let macro_export = if matches!(&item_trait.vis, Visibility::Inherited) {
        quote! {}
    } else {
        quote! {#[macro_export]}
    };

    for item in &mut item_trait.items {
        match item {
            TraitItem::Const(item) => {
                item.default = None;
            }
            TraitItem::Fn(item) => {
                item.default = None;
            }
            TraitItem::Type(item) => {
                item.default = None;
            }
            _ => (),
        }
    }

    let doc = format!(
        "Implement trait [`{ident}`]. Methods not specified will be forwarded to a field's implementation.\n# Syntax\n```\n# /*\n{name}!{{\n    @[field: T]\n    impl<T: {ident}> {ident} for MyType<T> {{\n        ..\n    }}\n}}\n# */\n```"
    );

    let mut trait_in = quote! {#item_trait};
    decratify(&mut trait_in);

    quote! {
        #trait_out

        #[allow(unused_macros)]
        #[doc = #doc]
        #macro_export
        macro_rules! #name {
            ($($tt: tt)*) => {
                const _: () = {
                    #(#[allow(unused_imports)] use #imports;)*
                    ::trait_deref::impl_trait! {
                        {#trait_in} {$($tt)*}
                    }
                };
            }
        }
    }
    .into()
}

struct ImplTraitInput {
    pub item_trait: ItemTrait,
    pub impl_block: ImplBlock,
}

struct ImplBlock {
    pub at_token: Token![@],
    pub bracket: Bracket,
    pub field: Ident,
    pub colon_token: Token![:],
    pub ty: Type,
    pub item_impl: ItemImpl,
}

impl Parse for ImplTraitInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let item_trait;
        let impl_block;
        braced!(item_trait in input);
        braced!(impl_block in input);
        Ok(ImplTraitInput {
            item_trait: item_trait.parse()?,
            impl_block: impl_block.parse()?,
        })
    }
}

impl Parse for ImplBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(ImplBlock {
            at_token: input.parse()?,
            bracket: bracketed!(content in input),
            field: content.parse()?,
            colon_token: content.parse()?,
            ty: content.parse()?,
            item_impl: input.parse()?,
        })
    }
}

/// Fill missing items in a trait.
#[doc(hidden)]
#[proc_macro]
pub fn impl_trait(tokens: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(tokens as ImplTraitInput);

    let mut impl_block = input.impl_block.item_impl;

    let field = input.impl_block.field;

    let inner_ty = input.impl_block.ty;

    let mut extended = Vec::new();

    let trait_name = input.item_trait.ident;

    for item in input.item_trait.items {
        match item {
            TraitItem::Const(item) => {
                if !impl_block.items.iter().any(|x| match x {
                    ImplItem::Const(v) => v.ident == item.ident,
                    _ => false,
                }) {
                    let ident = item.ident;
                    let ty = item.ty;
                    extended.push(parse_quote!(
                        const #ident: #ty = #inner_ty::#ident;
                    ));
                }
            }
            // rc mode
            TraitItem::Fn(item) if item.attrs.iter().any(|x| x.path().is_ident("rc")) => {
                if !impl_block.items.iter().any(|x| match x {
                    ImplItem::Fn(v) => v.sig.ident == item.sig.ident,
                    _ => false,
                }) {
                    let sig = &item.sig;
                    let ident = &sig.ident;
                    let this = match item.sig.inputs.get(0) {
                        Some(FnArg::Typed(arg)) => &arg.pat,
                        _ => panic!("Expected at least 2 items and no receiver."),
                    };
                    let func = match item.sig.inputs.get(1) {
                        Some(FnArg::Typed(arg)) => &arg.pat,
                        _ => panic!("Expected at least 2 items."),
                    };
                    let rest = item.sig.inputs.iter().skip(2).filter_map(|x| match x {
                        FnArg::Receiver(_) => None,
                        FnArg::Typed(x) => Some(&x.pat),
                    });
                    extended.push(parse_quote!(
                        #sig {
                            #trait_name::#ident(#this, |__x| &#func(__x).#field, #(#rest),*)
                        }
                    ));
                }
            }
            TraitItem::Fn(item) => {
                if !impl_block.items.iter().any(|x| match x {
                    ImplItem::Fn(v) => v.sig.ident == item.sig.ident,
                    _ => false,
                }) {
                    let sig = item.sig;
                    let ident = &sig.ident;
                    let names = sig.inputs.iter().filter_map(|x| match x {
                        FnArg::Receiver(_) => None,
                        FnArg::Typed(x) => Some(&x.pat),
                    });
                    let recv = match sig.receiver() {
                        None => continue,
                        Some(recv) => {
                            if recv.colon_token.is_some() {
                                continue;
                            }
                            if recv.reference.is_none() {
                                quote! {}
                            } else if recv.mutability.is_some() {
                                quote! {&mut}
                            } else {
                                quote! {&}
                            }
                        }
                    };
                    extended.push(parse_quote!(
                        #sig {
                            #trait_name::#ident(#recv self.#field, #(#names),*)
                        }
                    ));
                }
            }
            TraitItem::Type(item) => {
                if !impl_block.items.iter().any(|x| match x {
                    ImplItem::Type(v) => v.ident == item.ident,
                    _ => false,
                }) {
                    let ident = item.ident;
                    extended.push(parse_quote!(
                        type #ident = #inner_ty::#ident;
                    ));
                }
            }
            _ => (),
        }
    }

    impl_block.items.extend(extended);

    quote! {#impl_block}.into()
}
