use crate::hlist::{Cons, Nil};

/// --------------------------------------------
/// Compile-time type equality
/// --------------------------------------------
/// `<A as TypeEq<B>>::EQ` is `true` iff `A` and `B` are the same type. Stable
/// Rust cannot decide type *inequality*, so we use specialization: the blanket
/// impl answers `false`, and the reflexive `T == T` impl specializes it to
/// `true`.
pub trait TypeEq<U: ?Sized> {
    const EQ: bool;
}

impl<T: ?Sized, U: ?Sized> TypeEq<U> for T {
    default const EQ: bool = false;
}

impl<T: ?Sized> TypeEq<T> for T {
    const EQ: bool = true;
}

/// --------------------------------------------
/// Pairwise conflict rule
/// --------------------------------------------
pub trait ConflictsWith<T> {
    const VALUE: bool;
}

/// Read<A> vs Read<B> => shared access, never conflicts.
impl<A, B> ConflictsWith<crate::access::Read<B>> for crate::access::Read<A> {
    const VALUE: bool = false;
}

/// Read<A> vs Write<B> => conflict iff `A` and `B` are the same resource.
impl<A, B> ConflictsWith<crate::access::Write<B>> for crate::access::Read<A> {
    const VALUE: bool = <A as TypeEq<B>>::EQ;
}

/// Write<A> vs Read<B> => conflict iff `A` and `B` are the same resource.
impl<A, B> ConflictsWith<crate::access::Read<B>> for crate::access::Write<A> {
    const VALUE: bool = <A as TypeEq<B>>::EQ;
}

/// Write<A> vs Write<B> => conflict iff `A` and `B` are the same resource.
impl<A, B> ConflictsWith<crate::access::Write<B>> for crate::access::Write<A> {
    const VALUE: bool = <A as TypeEq<B>>::EQ;
}

/// --------------------------------------------
/// AnyConflict<H>
/// Checks if head H conflicts with ANY element in tail
/// --------------------------------------------
pub trait AnyConflict<H> {
    const VALUE: bool;
}

/// Base case: empty list => no conflicts
impl<H> AnyConflict<H> for Nil {
    const VALUE: bool = false;
}

/// Recursive case:
/// TH is head of list
/// TT is tail
impl<H, TH, TT> AnyConflict<H> for Cons<TH, TT>
where
    TH: ConflictsWith<H>,
    TT: AnyConflict<H>,
{
    const VALUE: bool =
        <TH as ConflictsWith<H>>::VALUE
        || <TT as AnyConflict<H>>::VALUE;
}

/// --------------------------------------------
/// NoConflicts (full list validation)
/// --------------------------------------------
pub trait NoConflicts {
    const VALUE: bool;
}

/// Empty list is valid
impl NoConflicts for Nil {
    const VALUE: bool = true;
}

/// Recursive rule:
/// - tail must be valid
/// - head must not conflict with tail
impl<H, T> NoConflicts for Cons<H, T>
where
    T: NoConflicts + AnyConflict<H>,
{
    const VALUE: bool =
        !<T as AnyConflict<H>>::VALUE
        && <T as NoConflicts>::VALUE;
}