//! The `ParamAccess` trait and its access-collection machinery.
//!
//! Every `ParamAccess` declares the set of resources it touches as a
//! self-contained, `Nil`-terminated type-level list of `Read`/`Write` markers
//! (built with the [`accesses!`](crate::accesses) macro). Tuples then
//! splice their members' lists together with the framework-internal [`Concat`]
//! operation, so an implementor never has to think about "the rest of the
//! params".

use crate::access::{Read, Write};
use crate::hlist::{AccessList, Cons, Nil};

/// A parameter's **access declaration**: the set of resources it touches.
///
/// This is the half of "being a parameter" that the conflict checker needs; a
/// behavioral trait (fetching the value, running, …) sits on top as
/// `YourParam: ParamAccess`. Implementors declare their accesses as a list,
/// e.g. `type Accesses = accesses![Read<Storage>, Write<Part<Storage, T>>];`
/// (usually via `#[derive(ParamAccess)]`); a parameter that touches nothing uses
/// `type Accesses = accesses![];`.
pub trait ParamAccess {
    /// This parameter's own accesses (a `Nil`-terminated type-level list).
    type Accesses: AccessList;
}

// --------------------------------------------------------------------------
// Structural impls provided by the framework.
// --------------------------------------------------------------------------

/// `Option<P>` has the exact same access footprint as `P`; whether the
/// parameter is present at runtime does not change what it could touch.
impl<P: ParamAccess> ParamAccess for Option<P> {
    type Accesses = P::Accesses;
}

/// A shared reference `&R` is a read of resource `R`.
impl<R> ParamAccess for &R {
    type Accesses = Cons<Read<R>, Nil>;
}

/// A mutable reference `&mut R` is a write of resource `R`.
impl<R> ParamAccess for &mut R {
    type Accesses = Cons<Write<R>, Nil>;
}

/// The empty tuple accesses nothing.
impl ParamAccess for () {
    type Accesses = Nil;
}

/// Right-folds the tuple members' access lists into one flattened list:
/// `A1 ++ (A2 ++ ( … ++ An))`.
macro_rules! tuple_access {
    ($last:ident) => { <$last as ParamAccess>::Accesses };
    ($head:ident, $($rest:ident),+) => {
        <<$head as ParamAccess>::Accesses as AccessList>::Concat<tuple_access!($($rest),+)>
    };
}

/// Generates `impl ParamAccess for (P1, …, Pn)` for a single arity. The
/// `AccessList::Concat` GAT is total, so no `where` bounds are needed.
macro_rules! impl_param_tuple {
    ($($param:ident),+) => {
        impl<$($param: ParamAccess),+> ParamAccess for ($($param,)+) {
            type Accesses = tuple_access!($($param),+);
        }
    };
}

impl_param_tuple!(P1);
impl_param_tuple!(P1, P2);
impl_param_tuple!(P1, P2, P3);
impl_param_tuple!(P1, P2, P3, P4);
impl_param_tuple!(P1, P2, P3, P4, P5);
