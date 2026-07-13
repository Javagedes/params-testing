//! Derive macros for the `params` crate.
//!
//! `#[derive(Resource)]` gives a type a compile-time structural identity
//! (the hidden `params::HasKey` + `params::HasPath` traits), derived from the
//! type's name with every generic type parameter folded in — so `Foo<A>` and
//! `Foo<B>` are distinct (each such parameter must itself have an identity).
//!
//! `#[derive(ParamAccess)]` additionally generates the `params::ParamAccess` impl from an
//! `#[accesses(read(...), write(...))]` attribute. A param's own generic scopes any
//! resource that doesn't already name it into a `Part<R, T>` partition.
//!
//! `resource_key!(i32, u32, ...)` gives external/primitive types just the
//! identity (no `HasPath`), so they can serve as partition keys or generic
//! arguments without a full derive.

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    DeriveInput, GenericParam, Generics, Ident, Token, Type, parse_macro_input, parse_quote,
};

/// FNV-1a 64-bit hash of a type's name. Distinct names almost always hash to
/// distinct values, which is what lets two identities be compared with a single
/// `==` on a `u64` const generic. Shared by every entry point so they all agree
/// on identity.
fn hash_name(name: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The identifiers of a type's generic *type* parameters (lifetimes and consts
/// skipped), in declaration order.
fn type_param_idents(generics: &Generics) -> Vec<Ident> {
    generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(t) => Some(t.ident.clone()),
            _ => None,
        })
        .collect()
}

/// Generates the identity impls (`HasKey` + `HasPath`) for a type, folding every
/// generic type parameter into the key so `Foo<A>` and `Foo<B>` are distinct
/// (each such parameter must itself have an identity / `HasKey`).
fn identity_impls(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let name_hash = hash_name(&name.to_string());

    let type_params = type_param_idents(&input.generics);

    let mut args = quote! { ::params::ANil };
    for tp in type_params.iter().rev() {
        args = quote! { ::params::ACons<<#tp as ::params::HasKey>::Key, #args> };
    }

    let mut generics = input.generics.clone();
    for param in &mut generics.params {
        if let GenericParam::Type(t) = param {
            t.bounds.push(parse_quote!(::params::HasKey));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let self_key = quote! { ::params::Sig<#name_hash, #args> };

    quote! {
        impl #impl_generics ::params::HasKey for #name #ty_generics #where_clause {
            type Key = #self_key;
        }
        impl #impl_generics ::params::HasPath for #name #ty_generics #where_clause {
            type Path = ::params::PCons<#self_key, ::params::PNil>;
        }
    }
}

/// Does `ty`'s token stream mention the identifier `ident` anywhere?
fn type_mentions(ty: &Type, ident: &Ident) -> bool {
    fn scan(ts: proc_macro2::TokenStream, ident: &Ident) -> bool {
        ts.into_iter().any(|tt| match tt {
            proc_macro2::TokenTree::Ident(id) => &id == ident,
            proc_macro2::TokenTree::Group(g) => scan(g.stream(), ident),
            _ => false,
        })
    }
    scan(quote! { #ty }, ident)
}

/// Auto-scopes a resource `R` into `Part<R, G>` for each of the param's own
/// generics `G` that `R` does not already mention — so `read(Storage)` on
/// `Config<T>` becomes `Read<Part<Storage, T>>`, while `read(Service<T>)` (which
/// already names `T`) stays `Read<Service<T>>`.
fn scope(ty: &Type, generics: &[Ident]) -> proc_macro2::TokenStream {
    let mut resource = quote! { #ty };
    for g in generics {
        if !type_mentions(ty, g) {
            resource = quote! { ::params::Part<#resource, #g> };
        }
    }
    resource
}

/// Parsed form of `#[accesses(read(A, B), write(C, D))]`.
struct AccessSpec {
    reads: Vec<Type>,
    writes: Vec<Type>,
}

impl Parse for AccessSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        while !input.is_empty() {
            let kind: Ident = input.parse()?;
            let content;
            syn::parenthesized!(content in input);
            let types = content.parse_terminated(Type::parse, Token![,])?;
            match kind.to_string().as_str() {
                "read" => reads.extend(types),
                "write" => writes.extend(types),
                other => {
                    return Err(syn::Error::new(
                        kind.span(),
                        format!("expected `read(...)` or `write(...)`, found `{other}`"),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(AccessSpec { reads, writes })
    }
}

/// Marks a type as a **resource** — a unit of data the conflict checker can
/// reason about — by giving it a structural identity (`HasKey` + `HasPath`) and
/// nothing else.
///
/// A resource is the *noun*; it declares no accesses of its own. It is
/// *requested* by reference — the blanket `impl ParamAccess for &R` / `&mut R`
/// makes `&Storage` a read and `&mut Storage` a write of it — or through a param
/// that names it (e.g. a `Config<T>` reading a `Part<Storage, T>`). For a type
/// that is itself a parameter, use `#[derive(ParamAccess)]` instead.
#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    identity_impls(&input).into()
}

/// Marks a type as a **parameter** — something a function requests — by
/// generating its `ParamAccess` impl (plus the identity, so a param may also be
/// named or nested like a resource).
///
/// A parameter is the *verb*: declare what it touches with
/// `#[accesses(read(A, B), write(C, D))]`. Each of the type's own generics
/// scopes the resources that don't name it into `Part<R, G>` partitions, so
/// `read(Storage)` on `Config<T>` reads just the `T`-partition of `Storage`.
/// For a plain data type that is only *accessed* (not a param), use
/// `#[derive(Resource)]`.
#[proc_macro_derive(ParamAccess, attributes(accesses))]
pub fn derive_param(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let identity = identity_impls(&input);

    let generic_idents = type_param_idents(&input.generics);

    let mut items: Vec<proc_macro2::TokenStream> = Vec::new();
    for attr in &input.attrs {
        if attr.path().is_ident("accesses") {
            let spec: AccessSpec = match attr.parse_args() {
                Ok(spec) => spec,
                Err(err) => return err.to_compile_error().into(),
            };
            for r in &spec.reads {
                let scoped = scope(r, &generic_idents);
                items.push(quote! { ::params::Read<#scoped> });
            }
            for w in &spec.writes {
                let scoped = scope(w, &generic_idents);
                items.push(quote! { ::params::Write<#scoped> });
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        #identity

        impl #impl_generics ::params::ParamAccess for #name #ty_generics #where_clause {
            type Accesses = ::params::accesses![#(#items),*];
        }
    }
    .into()
}

/// Gives external or primitive types (e.g. `i32`) a `params::HasKey` identity so
/// they can serve as partition keys or generic arguments. Accepts a
/// comma-separated list; each key is derived from the type's name (no numbers by
/// hand). These get no `HasPath`, so they can't be used as resources directly —
/// use `#[derive(Resource)]` / `#[derive(ParamAccess)]` for that.
#[proc_macro]
pub fn resource_key(input: TokenStream) -> TokenStream {
    let types = parse_macro_input!(input with Punctuated::<Type, Token![,]>::parse_terminated);
    let impls = types.iter().map(|ty| {
        let name: String = quote! { #ty }
            .to_string()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let name_hash = hash_name(&name);
        quote! {
            impl ::params::HasKey for #ty {
                type Key = ::params::Sig<#name_hash, ::params::ANil>;
            }
        }
    });

    quote! { #(#impls)* }.into()
}

/// Compile-time assertion that a comma-separated list of params has no access
/// conflict, pinpointing the offending *pair* — with 3+ params the failing check
/// names exactly the two that clash and points at your code.
///
/// ```ignore
/// assert_no_conflicts!(&Storage, Config<i32>, &mut Storage);
/// // error: `&Storage` and `&mut Storage` conflict: ...
/// ```
#[proc_macro]
pub fn assert_no_conflicts(input: TokenStream) -> TokenStream {
    let params: Vec<Type> =
        parse_macro_input!(input with Punctuated::<Type, Token![,]>::parse_terminated)
            .into_iter()
            .collect();

    let mut checks = Vec::new();

    // Each param must be internally consistent.
    for p in &params {
        let msg = format!("`{}` has a self-conflicting access set.", ty_str(p));
        checks.push(quote_spanned! { p.span() =>
            const { ::core::assert!(!::params::has_conflict::<(#p,)>(), #msg) };
        });
    }

    // Every pair, reported only when the conflict is *between* the two (an
    // internal conflict of either is named once, by the check above).
    for i in 0..params.len() {
        for j in (i + 1)..params.len() {
            let (pi, pj) = (&params[i], &params[j]);
            let msg = format!(
                "`{}` and `{}` conflict: they access overlapping data and at least one requires `write` access.",
                ty_str(pi),
                ty_str(pj),
            );
            checks.push(quote_spanned! { pj.span() =>
                const {
                    ::core::assert!(
                        !(
                            ::params::has_conflict::<(#pi, #pj)>()
                                && !::params::has_conflict::<(#pi,)>()
                                && !::params::has_conflict::<(#pj,)>()
                        ),
                        #msg
                    )
                };
            });
        }
    }

    quote! { #(#checks)* }.into()
}

/// Renders a type roughly as the user wrote it (token rendering with the spaces
/// that `proc_macro2` inserts around `<`, `>`, `,`, `::` cleaned up).
fn ty_str(ty: &Type) -> String {
    quote! { #ty }
        .to_string()
        .replace(" <", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace(" ::", "::")
        .replace(":: ", "::")
}
