//! The `Param` trait and its access-collection machinery.
//!
//! Every `Param` declares the set of resources it touches as a self-contained,
//! `Nil`-terminated type-level list of `Read`/`Write` markers (built with the
//! [`accesses!`](crate::accesses) macro). Tuples then splice their members'
//! lists together with the framework-internal [`Concat`] operation, so an
//! implementor never has to think about "the rest of the params".

use crate::hlist::{Concat, Nil};

/// A system parameter.
///
/// Implementors declare the resources they touch as an access list, e.g.
/// `type Access = accesses![Read<Storage>, Write<Config<T>>];`. A parameter
/// that touches nothing uses `type Access = accesses![];`.
pub trait Param {
    /// This parameter's own accesses (a `Nil`-terminated type-level list).
    type Access;
}

// --------------------------------------------------------------------------
// Structural impls provided by the framework.
// --------------------------------------------------------------------------

/// `Option<P>` has the exact same access footprint as `P`; whether the
/// parameter is present at runtime does not change what it could touch.
impl<P: Param> Param for Option<P> {
    type Access = P::Access;
}

/// The empty tuple accesses nothing.
impl Param for () {
    type Access = Nil;
}

impl<P1: Param> Param for (P1,) {
    type Access = P1::Access;
}

impl<P1: Param, P2: Param> Param for (P1, P2)
where
    P1::Access: Concat<P2::Access>,
{
    type Access = <P1::Access as Concat<P2::Access>>::Output;
}

impl<P1: Param, P2: Param, P3: Param> Param for (P1, P2, P3)
where
    P2::Access: Concat<P3::Access>,
    P1::Access: Concat<<P2::Access as Concat<P3::Access>>::Output>,
{
    type Access = <P1::Access as Concat<<P2::Access as Concat<P3::Access>>::Output>>::Output;
}

impl<P1: Param, P2: Param, P3: Param, P4: Param> Param for (P1, P2, P3, P4)
where
    P3::Access: Concat<P4::Access>,
    P2::Access: Concat<<P3::Access as Concat<P4::Access>>::Output>,
    P1::Access:
        Concat<<P2::Access as Concat<<P3::Access as Concat<P4::Access>>::Output>>::Output>,
{
    type Access = <P1::Access as Concat<
        <P2::Access as Concat<<P3::Access as Concat<P4::Access>>::Output>>::Output,
    >>::Output;
}

impl<P1: Param, P2: Param, P3: Param, P4: Param, P5: Param> Param for (P1, P2, P3, P4, P5)
where
    P4::Access: Concat<P5::Access>,
    P3::Access: Concat<<P4::Access as Concat<P5::Access>>::Output>,
    P2::Access:
        Concat<<P3::Access as Concat<<P4::Access as Concat<P5::Access>>::Output>>::Output>,
    P1::Access: Concat<
        <P2::Access as Concat<
            <P3::Access as Concat<<P4::Access as Concat<P5::Access>>::Output>>::Output,
        >>::Output,
    >,
{
    type Access = <P1::Access as Concat<
        <P2::Access as Concat<
            <P3::Access as Concat<<P4::Access as Concat<P5::Access>>::Output>>::Output,
        >>::Output,
    >>::Output;
}
