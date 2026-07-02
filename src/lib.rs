#![feature(specialization)]
#![allow(incomplete_features)]

mod access;
mod conflict_check;
mod hlist;
mod param;
mod system;

// -------------------------------------------------------------------------
// Public API: implement `Param` (declaring accesses with the `accesses!`
// macro over `Read`/`Write`), then call `assert_no_conflicts` at a boundary.
// -------------------------------------------------------------------------
pub use access::{Read, Write};
pub use param::Param;
pub use system::assert_no_conflicts;

// Hidden plumbing: reachable because the `accesses!` macro expansion and the
// `assert_no_conflicts` bound need to name it, but not part of the intended
// public API.
#[doc(hidden)]
pub use conflict_check::{AnyConflict, ConflictsWith, NoConflicts, TypeEq};
#[doc(hidden)]
pub use hlist::{Concat, Cons, Nil};

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use crate::accesses;
    use crate::access::{Read, Write};
    use crate::param::Param;
    use crate::system::assert_no_conflicts;

    // ----------------------------------------------------------------
    // Stand-in resource types. In the real crate these already exist;
    // only the `Param` impls below are what a user writes.
    // ----------------------------------------------------------------
    struct Storage;
    struct Commands<'a>(#[allow(dead_code)] &'a ());
    struct Service<T>(#[allow(dead_code)] core::marker::PhantomData<T>);
    struct Config<T>(#[allow(dead_code)] core::marker::PhantomData<T>);
    struct ConfigMut<T>(#[allow(dead_code)] core::marker::PhantomData<T>);
    struct Hob<T>(#[allow(dead_code)] core::marker::PhantomData<T>);

    struct FooCfg;
    struct BarCfg;

    // ----------------------------------------------------------------
    // User-defined `Param` impls: each declares its resource accesses.
    // ----------------------------------------------------------------
    impl Param for &Storage {
        type Access = accesses![Read<Storage>];
    }
    impl Param for &mut Storage {
        type Access = accesses![Write<Storage>];
    }
    impl Param for Commands<'_> {
        // Deferred command buffer: no direct resource access.
        type Access = accesses![];
    }
    impl<T> Param for Service<T> {
        type Access = accesses![Read<Service<T>>];
    }
    impl<T> Param for Config<T> {
        // A config lives inside Storage, so reading it also shared-reads
        // Storage. That makes it conflict with `&mut Storage` (Write<Storage>)
        // while staying compatible with `&Storage` and other reads.
        type Access = accesses![Read<Storage>, Read<Config<T>>];
    }
    impl<T> Param for ConfigMut<T> {
        // Mutating a config takes EXCLUSIVE (Write) access to Storage, plus its
        // own `Config<T>` slot. Because it writes Storage, it now conflicts with
        // *any* other Storage access: `&Storage`, `&mut Storage`, every `Config`,
        // and every other `ConfigMut` (regardless of type parameter).
        type Access = accesses![Write<Storage>, Write<Config<T>>];
    }
    impl<T> Param for Hob<T> {
        type Access = accesses![Read<Hob<T>>];
    }

    #[test]
    fn disjoint_resources_are_ok() {
        // &Storage, Config<Foo>, Hob<Bar>, Service<Foo> — all reads / distinct
        // resources, so no conflict. (Config shared-reads Storage, which is
        // compatible with the &Storage shared read.)
        assert_no_conflicts::<(&Storage, Config<FooCfg>, Hob<BarCfg>, Service<FooCfg>)>();
    }

    #[test]
    fn shared_reads_are_ok() {
        // Two readers of the same config do not conflict.
        assert_no_conflicts::<(Config<FooCfg>, Config<FooCfg>, Commands<'_>)>();
    }

    #[test]
    fn nested_and_optional_params_are_ok() {
        assert_no_conflicts::<(Option<Config<FooCfg>>, (&Storage, Service<BarCfg>))>();
    }

    // Uncomment any of these to see the compile-time conflict error:

    // #[test]
    // fn write_write_same_resource_conflicts() {
    //     assert_no_conflicts::<(ConfigMut<FooCfg>, ConfigMut<FooCfg>)>();
    // }

    // #[test]
    // fn read_write_same_resource_conflicts() {
    //     assert_no_conflicts::<(Config<FooCfg>, ConfigMut<FooCfg>)>();
    // }

    // #[test]
    // fn aliased_storage_conflicts() {
    //     assert_no_conflicts::<(&Storage, &mut Storage)>();
    // }

    // Because `ConfigMut` now writes Storage, all of these conflict too:

    // #[test]
    // fn mut_storage_conflicts_with_any_config() {
    //     assert_no_conflicts::<(&mut Storage, Config<FooCfg>)>();
    //     assert_no_conflicts::<(&mut Storage, ConfigMut<BarCfg>)>();
    // }

    // #[test]
    // fn shared_read_conflicts_with_config_mut() {
    //     // &Storage (Read<Storage>) vs ConfigMut (Write<Storage>) -> conflict.
    //     assert_no_conflicts::<(&Storage, ConfigMut<FooCfg>)>();
    // }

    // #[test]
    // fn two_different_config_muts_conflict() {
    //     // Both take Write<Storage>, so even different config types clash.
    //     assert_no_conflicts::<(ConfigMut<FooCfg>, ConfigMut<BarCfg>)>();
    // }
}







