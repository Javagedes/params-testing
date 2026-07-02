use crate::conflict_check::NoConflicts;
use crate::param::Param;

/// Statically asserts that the parameter `P` has no internal access conflicts.
///
/// Call this at any generic boundary where a `Param` is accepted (e.g. when
/// registering a system). If `P`'s flattened access list contains a conflict,
/// the crate fails to compile at the instantiation site.
///
/// The check lives in an inline `const` block, so it is evaluated once per
/// concrete `P` at monomorphization — a conflict becomes a compile error, not
/// a runtime panic.
pub fn assert_no_conflicts<P>()
where
    P: Param,
    <P as Param>::Access: NoConflicts,
{
    const {
        assert!(
            <<P as Param>::Access as NoConflicts>::VALUE,
            "parameter access conflict: two parameters access the same resource and at least one is a `&mut` (Write) access",
        )
    }
}
