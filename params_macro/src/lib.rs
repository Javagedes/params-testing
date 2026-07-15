//! Procedural macros for the `params` crate.
//!
//! `#[derive(Resource)]` gives a type a compile-time structural identity
//! (the hidden `params::HasKey` + `params::HasPath` traits), derived from the
//! type's name with every generic type parameter folded in — so `Foo<A>` and
//! `Foo<B>` are distinct (each such parameter must itself have an identity).
//!
//! `#[accesses(...)]` is an attribute macro that generates the `params::Access`
//! impl (plus the identity). List the other params this one accesses: `&R` /
//! `&mut R` is a read / write of resource `R` (a generic auto-scopes it into a
//! `Part<R, T>` partition), and any other param has its footprint spliced in
//! as-is.

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

/// Parsed form of `#[accesses(&R, &mut R, OtherParam, ...)]`: the flat list of
/// params this one accesses.
struct AccessSpec {
    entries: Vec<Type>,
}

impl Parse for AccessSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let entries = Punctuated::<Type, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect();
        Ok(AccessSpec { entries })
    }
}

/// Marks a type as a **resource** — a unit of data the conflict checker can
/// reason about — by giving it a structural identity (`HasKey` + `HasPath`) and
/// nothing else.
///
/// A resource is the *noun*; it declares no accesses of its own. It is
/// *requested* by reference — the blanket `impl Access for &R` / `&mut R`
/// makes `&Storage` a read and `&mut Storage` a write of it — or through a param
/// that names it (e.g. a `Config<T>` reading a `Part<Storage, T>`). For a type
/// that is itself a parameter, use `#[accesses(...)]` instead.
#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    identity_impls(&input).into()
}

/// Marks a type as a **parameter** — something a function requests — by
/// generating its `Access` impl (plus the identity, so a param may also be
/// named or nested like a resource).
///
/// Declare the other params it accesses with `#[accesses(...)]`:
/// - `&R` / `&mut R` — a read / write of resource `R`; each of the type's own
///   generics auto-scopes it into a `Part<R, G>` partition (so `#[accesses(&Storage)]`
///   on `Config<T>` reads just the `T`-partition of `Storage`).
/// - any other param `P` — its own footprint is spliced in as-is.
///
/// For a plain data type that is only *accessed* (not a param), use
/// `#[derive(Resource)]`.
#[proc_macro_attribute]
pub fn accesses(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let spec = parse_macro_input!(attr as AccessSpec);
    let name = &input.ident;

    let identity = identity_impls(&input);
    let generic_idents = type_param_idents(&input.generics);

    // Build the access list from the declared params. A `&R` / `&mut R` entry is
    // a read / write of `R`, auto-scoped into the type's own generics; any other
    // entry is a param whose own footprint is spliced in as-is.
    let mut list = quote! { ::params::Nil };
    for entry in spec.entries.iter().rev() {
        if let Type::Reference(reference) = entry {
            let scoped = scope(reference.elem.as_ref(), &generic_idents);
            if reference.mutability.is_some() {
                list = quote! { ::params::Cons<::params::Write<#scoped>, #list> };
            } else {
                list = quote! { ::params::Cons<::params::Read<#scoped>, #list> };
            }
        } else {
            list = quote! {
                <<#entry as ::params::Access>::Accesses as ::params::AccessList>::Concat<#list>
            };
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        #input

        #identity

        impl #impl_generics ::params::Access for #name #ty_generics #where_clause {
            type Accesses = #list;
        }
    }
    .into()
}

/// Compile-time assertion that a comma-separated list of params has no access
/// conflict, pinpointing the offending *pair* — with 3+ params the failing check
/// names exactly the two that clash and points at your code.
///
/// ```ignore
/// assert_no_conflicts!(&Storage, Timer, &mut Storage);
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
