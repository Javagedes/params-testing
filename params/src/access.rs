//! Access markers. An access list ([`accesses!`](crate::accesses)) is a
//! type-level list of [`Read`]/[`Write`] markers over resources; [`Part`] names
//! a `Key`-indexed sub-region of a resource for partitioned access.

use core::marker::PhantomData;

/// A shared (read) access to resource `T`.
pub struct Read<T>(PhantomData<T>);

/// An exclusive (write) access to resource `T`.
pub struct Write<T>(PhantomData<T>);

/// A `Key`-indexed partition of resource `R`: `Read<Part<Storage, T>>` reads
/// only the `T`-region of `Storage`, `Write<Part<Storage, T>>` writes it. A
/// whole-`R` access (`Read<Storage>`) overlaps every partition, but
/// `Part<R, A>` and `Part<R, B>` are disjoint whenever `A` and `B` are
/// different resources. `R` must be a resource (`#[derive(Resource)]`) and
/// `Key` must have an identity (`#[derive(Resource)]` or `resource_key!`).
pub struct Part<R, K>(PhantomData<(R, K)>);
