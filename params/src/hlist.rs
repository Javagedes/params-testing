use core::marker::PhantomData;

/// The empty type-level list.
pub struct Nil;

/// A type-level list cell: head `H` followed by tail `T`.
pub struct Cons<H, T>(PhantomData<(H, T)>);

/// A type-level access list that knows how to concatenate with another list.
///
/// Every `Access::Accesses` is an `AccessList` (a `Cons`/`Nil` chain). Making
/// concatenation a total GAT on this trait is what lets the tuple `Access`
/// impls splice their members' lists together with no `where` bounds: the
/// `Concat` output is itself an `AccessList`, so folds nest freely. The `Nil` base
/// case is where "continue on to the rest of the params" happens — reaching the
/// end of the left list grafts the entire right list on.
pub trait AccessList {
    /// This list with `Rhs` appended onto its end.
    type Concat<Rhs: AccessList>: AccessList;
}

impl AccessList for Nil {
    type Concat<Rhs: AccessList> = Rhs;
}

impl<H, T: AccessList> AccessList for Cons<H, T> {
    type Concat<Rhs: AccessList> = Cons<H, T::Concat<Rhs>>;
}
