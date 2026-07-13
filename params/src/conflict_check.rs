use crate::hlist::{Cons, Nil};
use core::marker::PhantomData;

/// Compile-time type identity (specialization-free).
///
/// Stable Rust cannot decide type *inequality* by trait resolution, so instead
/// of comparing resource types directly we compare their structural *keys*.
///
/// A key is a [`Sig`] pairing a `u64` hash of the resource's name with the keys
/// of its generic arguments (an [`ACons`]/[`ANil`] list). Every resource
/// declares its key via [`HasKey`], which the derive macros generate from the
/// (hashed) type name and its parameters. [`KeyEq`] decides equality by
/// comparing the name hashes with `==` (a `const` on `u64` const generics) and
/// recursing over the closed `{Sig, ANil, ACons}` argument constructors — each
/// impl is selected by a distinct pair of head constructors, so none overlap
/// and no specialization is required.
///
/// Two *different* names could in principle hash-collide; that would over-report
/// a conflict (a spurious error), never miss one, and is astronomically
/// unlikely for a 64-bit hash. Identical names always hash identically, so real
/// conflicts are never lost.
///
/// `<A::Key as KeyEq<B::Key>>::EQ` is therefore `true` iff `A` and `B` name the
/// same resource.

/// The empty argument list.
pub struct ANil;
/// An argument-list cell: head key `S` followed by tail list `R`.
pub struct ACons<S, R>(PhantomData<(S, R)>);
/// A resource key: a `u64` name hash paired with its arguments (key list).
pub struct Sig<const NAME: u64, Args>(PhantomData<Args>);

/// Associates a resource type with its structural [key](Sig).
///
/// Implemented by the derive macros; not meant to be written by hand.
pub trait HasKey {
    type Key;
}

/// Structural equality over [keys](Sig). `EQ` is `true` iff the two keys are
/// identical.
pub trait KeyEq<Rhs> {
    const EQ: bool;
}

// -- signature: equal iff the name hashes match and the arguments match -------
impl<A1, A2, const N1: u64, const N2: u64> KeyEq<Sig<N2, A2>> for Sig<N1, A1>
where
    A1: KeyEq<A2>,
{
    const EQ: bool = N1 == N2 && <A1 as KeyEq<A2>>::EQ;
}

// -- argument list ----------------------------------------------------------
impl KeyEq<ANil> for ANil {
    const EQ: bool = true;
}
impl<S, R> KeyEq<ANil> for ACons<S, R> {
    const EQ: bool = false;
}
impl<S, R> KeyEq<ACons<S, R>> for ANil {
    const EQ: bool = false;
}
impl<S1, R1, S2, R2> KeyEq<ACons<S2, R2>> for ACons<S1, R1>
where
    S1: KeyEq<S2>,
    R1: KeyEq<R2>,
{
    const EQ: bool = <S1 as KeyEq<S2>>::EQ && <R1 as KeyEq<R2>>::EQ;
}

/// Resource containment (paths).
///
/// Beyond bare identity, a resource may be *nested inside* another: e.g. a
/// `Config<T>` slot lives within `Storage`. Accessing the child therefore
/// overlaps any access to an ancestor, while two distinct children (a different
/// `T`) stay disjoint from each other.
///
/// Each resource exposes a [`HasPath::Path`] — the chain of [keys](Sig) from the
/// root ancestor down to and including itself, as a [`PCons`]/[`PNil`] list.
/// Two resources *overlap* iff one path is a prefix of the other, decided by
/// [`PathOverlap`] comparing elements with [`KeyEq`].

/// The empty path.
pub struct PNil;
/// A path cell: ancestor key `H` followed by the rest of the path `T`.
pub struct PCons<H, T>(PhantomData<(H, T)>);

/// A resource-containment path (a [`PCons`]/[`PNil`] chain, root first) that can
/// be extended at its deepest end.
pub trait PathList {
    /// This path with key `K` appended as the new deepest element.
    type Push<K>: PathList;
}

impl PathList for PNil {
    type Push<K> = PCons<K, PNil>;
}

impl<H, T: PathList> PathList for PCons<H, T> {
    type Push<K> = PCons<H, T::Push<K>>;
}

/// Associates a resource with its containment [path](PathList), from the root
/// ancestor down to itself. Implemented by `#[derive(Resource)]`.
pub trait HasPath {
    type Path: PathList;
}

/// A partition `Part<R, K>` sits one level below `R`: its path is `R`'s path
/// with the partition key appended, so it overlaps any whole-`R` access but is
/// disjoint from partitions with a different key.
impl<R, K> HasPath for crate::access::Part<R, K>
where
    R: HasPath,
    K: HasKey,
{
    type Path = <<R as HasPath>::Path as PathList>::Push<<K as HasKey>::Key>;
}

/// `VALUE` is `true` iff the two paths overlap — i.e. one is a prefix of the
/// other, so the resources are identical or one contains the other.
pub trait PathOverlap<Other> {
    const VALUE: bool;
}

// An empty path is a prefix of any path (and vice versa): they overlap.
impl PathOverlap<PNil> for PNil {
    const VALUE: bool = true;
}
impl<H, T> PathOverlap<PCons<H, T>> for PNil {
    const VALUE: bool = true;
}
impl<H, T> PathOverlap<PNil> for PCons<H, T> {
    const VALUE: bool = true;
}
// Two non-empty paths overlap iff their heads name the same resource and the
// remainders still overlap; diverging heads mean disjoint subtrees.
impl<H1, T1, H2, T2> PathOverlap<PCons<H2, T2>> for PCons<H1, T1>
where
    H1: KeyEq<H2>,
    T1: PathOverlap<T2>,
{
    const VALUE: bool = <H1 as KeyEq<H2>>::EQ && <T1 as PathOverlap<T2>>::VALUE;
}

/// Pairwise conflict rule: two accesses conflict iff their resources *overlap*
/// (are equal, or one contains the other — see [`PathOverlap`]) and at least one
/// is a `Write`.
pub trait ConflictsWith<T> {
    const VALUE: bool;
}

/// `Read` vs `Read` => two shared reads, never conflict.
impl<A, B> ConflictsWith<crate::access::Read<B>> for crate::access::Read<A> {
    const VALUE: bool = false;
}

/// `Read<A>` vs `Write<B>` => conflict iff `A` and `B` overlap.
impl<A, B> ConflictsWith<crate::access::Write<B>> for crate::access::Read<A>
where
    A: HasPath,
    B: HasPath,
    <A as HasPath>::Path: PathOverlap<<B as HasPath>::Path>,
{
    const VALUE: bool = <<A as HasPath>::Path as PathOverlap<<B as HasPath>::Path>>::VALUE;
}

/// `Write<A>` vs `Read<B>` => conflict iff `A` and `B` overlap.
impl<A, B> ConflictsWith<crate::access::Read<B>> for crate::access::Write<A>
where
    A: HasPath,
    B: HasPath,
    <A as HasPath>::Path: PathOverlap<<B as HasPath>::Path>,
{
    const VALUE: bool = <<A as HasPath>::Path as PathOverlap<<B as HasPath>::Path>>::VALUE;
}

/// `Write<A>` vs `Write<B>` => two writes, conflict iff `A` and `B` overlap.
impl<A, B> ConflictsWith<crate::access::Write<B>> for crate::access::Write<A>
where
    A: HasPath,
    B: HasPath,
    <A as HasPath>::Path: PathOverlap<<B as HasPath>::Path>,
{
    const VALUE: bool = <<A as HasPath>::Path as PathOverlap<<B as HasPath>::Path>>::VALUE;
}

/// `VALUE` is `true` iff access `H` conflicts with any element of the list.
pub trait AnyConflict<H> {
    const VALUE: bool;
}

/// Nothing to conflict with in an empty list.
impl<H> AnyConflict<H> for Nil {
    const VALUE: bool = false;
}

/// `H` conflicts with the list if it conflicts with the head or with any element
/// of the tail.
impl<H, TH, TT> AnyConflict<H> for Cons<TH, TT>
where
    TH: ConflictsWith<H>,
    TT: AnyConflict<H>,
{
    const VALUE: bool = <TH as ConflictsWith<H>>::VALUE || <TT as AnyConflict<H>>::VALUE;
}

/// `VALUE` is `true` iff the whole access list is internally conflict-free.
pub trait NoConflicts {
    const VALUE: bool;
}

/// An empty list is trivially conflict-free.
impl NoConflicts for Nil {
    const VALUE: bool = true;
}

/// A list is conflict-free iff its head clashes with nothing in the tail and the
/// tail is itself conflict-free.
impl<H, T> NoConflicts for Cons<H, T>
where
    T: NoConflicts + AnyConflict<H>,
{
    const VALUE: bool = !<T as AnyConflict<H>>::VALUE && <T as NoConflicts>::VALUE;
}
