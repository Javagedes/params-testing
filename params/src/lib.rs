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
pub use params_macro::{Resource, accesses, assert_no_conflicts};

/// Returns `true` iff `P`'s flattened access list contains a conflict.
pub const fn has_conflict<P>() -> bool
where
    P: Access,
    <P as Access>::Accesses: NoConflicts,
{
    !<<P as Access>::Accesses as NoConflicts>::VALUE
}

// Hidden plumbing: reachable because the `#[accesses(...)]` / `#[derive(Resource)]`
// output and the `assert_no_conflicts` bound need to name it, but not part of the
// intended public API.
#[doc(hidden)]
pub use conflict_check::{
    ACons, ANil, AnyConflict, ConflictsWith, HasKey, HasPath, KeyEq, NoConflicts, PCons, PNil,
    PathList, PathOverlap, Sig,
};
#[doc(hidden)]
pub use hlist::{AccessList, Cons, Nil};

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use crate::{Resource, accesses, has_conflict};

    // A *resource* is data (the noun) with a structural identity; a *param* is a
    // request (the verb) that declares what it touches with
    // `#[accesses(read(..), write(..))]`. `Storage` is data, requested by
    // reference — `&Storage` (read) / `&mut Storage` (write) are params via a
    // blanket impl — while `Config`/`ConfigMut`/`Hob` map onto a resource. A
    // param's own generic auto-scopes a whole-resource access into a `Part<_, T>`
    // partition, so `Config<KeyA>` and `Config<KeyB>` stay disjoint (the keys are
    // themselves resources).

    #[derive(Resource)]
    struct KeyA;

    #[derive(Resource)]
    struct KeyB;

    #[accesses]
    struct Storage;

    #[accesses(write(Commands<'a>))]
    struct Commands<'a>(#[allow(dead_code)] &'a ());

    #[accesses(read(Service<T>))]
    struct Service<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    #[accesses(read(Storage))]
    struct Config<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    #[accesses(write(Storage))]
    struct ConfigMut<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    #[accesses(read(Hob<T>))]
    struct Hob<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

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
        // Commands should conflict with itself (Write<Commands> vs Write<Commands>).
        assert!(has_conflict::<(Commands<'_>, Commands<'_>)>());
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
