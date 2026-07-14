//! The `Access` trait and its access-collection machinery.
//!
//! Every `Access` declares the set of resources it touches as a
//! self-contained, `Nil`-terminated type-level list of `Read`/`Write` markers
//! (normally via the `#[accesses(...)]` attribute). Tuples then splice their
//! members' lists together with the framework-internal [`AccessList::Concat`]
//! operation, so an implementor never has to think about "the rest of the
//! params".

use crate::conflict_check::{Read, Write};
use crate::hlist::{AccessList, Cons, Nil};

/// A parameter's **access declaration**: the set of resources it touches.
///
/// This is the half of "being a parameter" that the conflict checker needs; a
/// behavioral trait (fetching the value, running, …) sits on top as
/// `YourParam: Access`. Implementors normally declare their accesses with
/// the `#[accesses(read(...), write(...))]` attribute, which builds a `Cons`/`Nil`
/// list of `Read`/`Write` markers; a parameter that touches nothing gets `Nil`.
pub trait Access {
    /// This parameter's own accesses (a `Nil`-terminated type-level list).
    type Accesses: AccessList;
}

// --------------------------------------------------------------------------
// Structural impls provided by the framework.
// --------------------------------------------------------------------------

/// `Option<P>` has the exact same access footprint as `P`; whether the
/// parameter is present at runtime does not change what it could touch.
impl<P: Access> Access for Option<P> {
    type Accesses = P::Accesses;
}

/// A shared reference `&R` is a read of resource `R`.
impl<R> Access for &R {
    type Accesses = Cons<Read<R>, Nil>;
}

/// A mutable reference `&mut R` is a write of resource `R`.
impl<R> Access for &mut R {
    type Accesses = Cons<Write<R>, Nil>;
}

/// The empty tuple accesses nothing.
impl Access for () {
    type Accesses = Nil;
}

/// Right-folds the tuple members' access lists into one flattened list:
/// `A1 ++ (A2 ++ ( … ++ An))`.
macro_rules! tuple_access {
    ($last:ident) => { <$last as Access>::Accesses };
    ($head:ident, $($rest:ident),+) => {
        <<$head as Access>::Accesses as AccessList>::Concat<tuple_access!($($rest),+)>
    };
}

/// Generates `impl Access for (P1, …, Pn)` for a single arity. The
/// `AccessList::Concat` GAT is total, so no `where` bounds are needed.
macro_rules! impl_param_tuple {
    ($($param:ident),+) => {
        impl<$($param: Access),+> Access for ($($param,)+) {
            type Accesses = tuple_access!($($param),+);
        }
    };
}

impl_param_tuple!(P1);
impl_param_tuple!(P1, P2);
impl_param_tuple!(P1, P2, P3);
impl_param_tuple!(P1, P2, P3, P4);
impl_param_tuple!(P1, P2, P3, P4, P5);
