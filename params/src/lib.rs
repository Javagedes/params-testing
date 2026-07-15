#![feature(const_type_name)]

mod conflict_check;
mod hlist;
mod param;

extern crate self as params;

// -------------------------------------------------------------------------
// Public API: declare a type's accesses with the `#[accesses(read(...),
// write(...))]` attribute (or implement `Access` by hand), then call
// `assert_no_conflicts` at a boundary.
// -------------------------------------------------------------------------
pub use conflict_check::{Part, Read, Write};
pub use param::Access;
pub use params_macro::{accesses, assert_no_conflicts};

/// Returns `true` iff `P`'s flattened access list contains a conflict.
pub const fn has_conflict<P>() -> bool
where
    P: Access,
    <P as Access>::Accesses: NoConflicts,
{
    !<<P as Access>::Accesses as NoConflicts>::VALUE
}

// Hidden plumbing: reachable because the `#[accesses(...)]` output and the
// `assert_no_conflicts` bound need to name it, but not part of the intended
// public API.
#[doc(hidden)]
pub use conflict_check::{
    AnyConflict, ConflictsWith, HasPath, KeyEq, NoConflicts, PCons, PNil, PathList, PathOverlap,
};
#[doc(hidden)]
pub use hlist::{AccessList, Cons, Nil};

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use crate::{accesses, has_conflict};

    struct KeyA;

    struct KeyB;

    // Has no accesses of its own, because it is the underlying storage for all other params.
    #[accesses]
    struct Storage;

    // Example of a param that only conflicts with itself
    #[accesses(&mut Self)]
    struct Commands<'a>(#[allow(dead_code)] &'a ());

    // Example of a param that does not conflict with anything
    #[accesses]
    struct Service<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    // Example of a param that has read access to a partition of storage, scoped by its own generic
    #[accesses(&Storage)]
    struct Config<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    // Example of a param that has write access to a partition of storage, scoped by its own generic
    #[accesses(&mut Storage)]
    struct ConfigMut<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    // Example of a param that does not conflict with anything
    #[accesses]
    struct Hob<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    // An example of a invalid param that conflicts with itself. Attempting to use it in a component will always fail to compile.
    #[accesses(&'a mut Storage, ConfigMut<T>)]
    struct Multi<'a, T>(
        #[allow(dead_code)] core::marker::PhantomData<T>,
        #[allow(dead_code)] &'a (),
    );

    #[test]
    fn test_config_param_success_scenarios() {
        assert!(!has_conflict::<(Config<KeyA>, &Storage)>());
        assert!(!has_conflict::<(&Storage, Config<KeyA>)>());

        assert!(!has_conflict::<(Config<KeyA>, Config<KeyA>)>());

        assert!(!has_conflict::<(Config<KeyA>, ConfigMut<KeyB>)>());
        assert!(!has_conflict::<(ConfigMut<KeyA>, Config<KeyB>)>());

        assert!(!has_conflict::<(ConfigMut<KeyB>, Config<KeyA>)>());
        assert!(!has_conflict::<(Config<KeyB>, ConfigMut<KeyA>)>());
    }

    #[test]
    fn test_config_param_conflict_scenarios() {
        assert!(has_conflict::<(Config<KeyA>, &mut Storage)>());
        assert!(has_conflict::<(&mut Storage, Config<KeyA>)>());

        assert!(has_conflict::<(Config<KeyA>, ConfigMut<KeyA>)>());
        assert!(has_conflict::<(ConfigMut<KeyA>, Config<KeyA>)>());

        assert!(has_conflict::<Multi<'_, KeyA>>());
    }

    #[test]
    fn test_config_mut_param_conflict_scenarios() {
        assert!(has_conflict::<(ConfigMut<KeyA>, &mut Storage)>());
        assert!(has_conflict::<(&mut Storage, ConfigMut<KeyA>)>());

        assert!(has_conflict::<(ConfigMut<KeyA>, &Storage)>());
        assert!(has_conflict::<(&Storage, ConfigMut<KeyA>)>());

        assert!(has_conflict::<(ConfigMut<KeyA>, Config<KeyA>)>());
        assert!(has_conflict::<(Config<KeyA>, ConfigMut<KeyA>)>());

        assert!(has_conflict::<(ConfigMut<KeyA>, ConfigMut<KeyA>)>());
    }

    #[test]
    fn test_commands_conflict_scenarios() {
        // Commands writes its own resource, so it conflicts only with itself.
        assert!(has_conflict::<(Commands<'_>, Commands<'_>)>());

        assert!(!has_conflict::<(Commands<'_>, Config<KeyA>)>());
        assert!(!has_conflict::<(Commands<'_>, &mut Storage)>());
    }

    #[test]
    fn test_option_conflict_scenarios() {
        assert!(has_conflict::<(Option<Config<KeyA>>, ConfigMut<KeyA>)>());
        assert!(has_conflict::<(ConfigMut<KeyA>, Option<Config<KeyA>>)>());

        assert!(has_conflict::<(ConfigMut<KeyA>, Option<ConfigMut<KeyA>>)>());

        assert!(!has_conflict::<(Option<Config<KeyA>>, Config<KeyA>)>());
        assert!(!has_conflict::<(Config<KeyA>, Option<Config<KeyA>>)>());
    }
}
