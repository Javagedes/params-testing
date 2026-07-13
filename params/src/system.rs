use crate::conflict_check::NoConflicts;
use crate::param::ParamAccess;

/// Returns `true` iff `P`'s flattened access list contains a conflict.
///
/// A `const fn`, so it works both at runtime and inside a `const {}` block —
/// e.g. a proc-macro emitting a compile-time check can write
/// `const { ::core::assert!(!params::has_conflict::<P>(), "…") }`.
pub const fn has_conflict<P>() -> bool
where
    P: ParamAccess,
    <P as ParamAccess>::Accesses: NoConflicts,
{
    !<<P as ParamAccess>::Accesses as NoConflicts>::VALUE
}

/// Asserts at runtime that `P` **has** an access conflict, panicking if it does
/// not.
///
/// This is the check a compile-time assertion cannot express: a genuine
/// conflict makes [`assert_no_conflicts!`](crate::assert_no_conflicts) a
/// *compile* error, so to test that the checker rejects a combination, assert it
/// at runtime — e.g.
/// `assert_conflict::<(&Storage, &mut Storage)>()`.
pub fn assert_conflict<P>()
where
    P: ParamAccess,
    <P as ParamAccess>::Accesses: NoConflicts,
{
    assert!(
        has_conflict::<P>(),
        "expected a parameter access conflict, but the accesses are disjoint",
    );
}

/// Asserts at runtime that `P` has **no** access conflict, panicking if it does.
///
/// The runtime counterpart to the compile-time
/// [`assert_no_conflicts!`](crate::assert_no_conflicts) (plural), useful for
/// keeping conflict and no-conflict cases together as ordinary tests.
pub fn assert_no_conflict<P>()
where
    P: ParamAccess,
    <P as ParamAccess>::Accesses: NoConflicts,
{
    assert!(
        !has_conflict::<P>(),
        "unexpected parameter access conflict: two parameters access overlapping data and at least one requires `write` access",
    );
}
