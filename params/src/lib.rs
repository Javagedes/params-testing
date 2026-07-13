// The `#[derive(Resource)]` macro expands to paths rooted at `::params`, so the
// crate needs to be able to name itself when the derive is used internally.
extern crate self as params;

mod access;
mod conflict_check;
mod hlist;
mod param;
mod system;

// -------------------------------------------------------------------------
// Public API: implement `ParamAccess` (declaring accesses with the `accesses!`
// macro over `Read`/`Write`, or `#[derive(ParamAccess)]` + `#[accesses(...)]`),
// then call `assert_no_conflicts` at a boundary.
// -------------------------------------------------------------------------
pub use access::{Part, Read, Write};
pub use param::ParamAccess;
pub use params_macro::{ParamAccess, Resource, assert_no_conflicts, resource_key};
pub use system::{assert_conflict, assert_no_conflict, has_conflict};

// Hidden plumbing: reachable because the `accesses!` macro, the
// `#[derive(Resource)]` output, and the `assert_no_conflicts` bound need to
// name it, but not part of the intended public API.
#[doc(hidden)]
pub use conflict_check::{
    ACons, ANil, AnyConflict, ConflictsWith, HasKey, HasPath, KeyEq, NoConflicts, PCons, PNil,
    PathList, PathOverlap, Sig,
};
#[doc(hidden)]
pub use hlist::{AccessList, Cons, Nil};

// Common primitive types ship with an identity so they can be used as partition
// keys or generic arguments with no annotation.
resource_key!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, bool, char
);

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use crate::system::{assert_conflict, assert_no_conflict};
    use crate::{ParamAccess, Resource, assert_no_conflicts};

    // Two roles. A *resource* is data (the noun): `#[derive(Resource)]` gives it
    // an identity but no accesses. A *param* is a request (the verb):
    // `#[derive(ParamAccess)] #[accesses(read(..), write(..))]` declares what it
    // touches. `Storage` is therefore a plain resource; it is *requested* by
    // reference — `&Storage` (read) / `&mut Storage` (write) are params via a
    // blanket impl — while `Config`/`ConfigMut`/`Hob`/… are params that map onto
    // a resource. A param's own generic auto-scopes a whole-resource access into
    // a `Part<_, T>` partition, so `Config<i32>` and `Config<u32>` stay disjoint.
    // Primitive keys like `i32` ship with the crate.
    #[derive(Resource)]
    struct Storage;

    #[derive(ParamAccess)]
    #[accesses(write(Commands<'a>))]
    struct Commands<'a>(#[allow(dead_code)] &'a ());

    #[derive(ParamAccess)]
    #[accesses(read(Service<T>))]
    struct Service<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    #[derive(ParamAccess)]
    #[accesses(read(Storage))]
    struct Config<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    #[derive(ParamAccess)]
    #[accesses(write(Storage))]
    struct ConfigMut<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    #[derive(ParamAccess)]
    #[accesses(read(Hob<T>))]
    struct Hob<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    #[test]
    fn test_config_param_success_scenarios() {
        assert_no_conflict::<(Config<i32>, &Storage)>();
        assert_no_conflict::<(&Storage, Config<i32>)>();

        assert_no_conflict::<(Config<i32>, Config<i32>)>();

        assert_no_conflict::<(Config<i32>, ConfigMut<u32>)>();
        assert_no_conflict::<(ConfigMut<i32>, Config<u32>)>();

        assert_no_conflict::<(ConfigMut<u32>, Config<i32>)>();
        assert_no_conflict::<(Config<u32>, ConfigMut<i32>)>();
    }

    #[test]
    fn test_config_param_conflict_scenarios() {
        assert_conflict::<(Config<i32>, &mut Storage)>();
        assert_conflict::<(&mut Storage, Config<i32>)>();

        assert_conflict::<(Config<i32>, ConfigMut<i32>)>();
        assert_conflict::<(ConfigMut<i32>, Config<i32>)>();
    }

    #[test]
    fn test_config_mut_param_conflict_scenarios() {
        assert_conflict::<(ConfigMut<i32>, &mut Storage)>();
        assert_conflict::<(&mut Storage, ConfigMut<i32>)>();

        assert_conflict::<(ConfigMut<i32>, &Storage)>();
        assert_conflict::<(&Storage, ConfigMut<i32>)>();

        assert_conflict::<(ConfigMut<i32>, Config<i32>)>();
        assert_conflict::<(Config<i32>, ConfigMut<i32>)>();

        assert_conflict::<(ConfigMut<i32>, ConfigMut<i32>)>();
    }

    #[test]
    fn test_hob() {
        // Pairwise pinpointing: with a comma-separated list, a conflict names the
        // exact two params. Compiles because these three are mutually disjoint.
        assert_no_conflicts!(Config<i32>, ConfigMut<u32>, Hob<i32>);

        // The tuple form checks the whole set at once.
        assert_no_conflicts!((Config<i32>, ConfigMut<u32>));

        // The const-fn form a `#[component]` macro could emit for its own message.
        const { assert!(!crate::has_conflict::<(Config<i32>, ConfigMut<u32>)>()) }

        // Uncomment to see the pairwise error point at the offending param:
        // assert_no_conflicts!(Config<u32>, ConfigMut<i32>, ConfigMut<u32>);
    }

    #[test]
    fn test_commands_conflict_scenarios() {
        // Commands should conflict with itself (Write<Commands> vs Write<Commands>).
        assert_conflict::<(Commands<'_>, Commands<'_>)>();
    }
}
