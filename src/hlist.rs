use core::marker::PhantomData;

/// The empty type-level list.
pub struct Nil;

/// A type-level list cell: head `H` followed by tail `T`.
pub struct Cons<H, T>(PhantomData<(H, T)>);

/// Type-level list concatenation: appends `Rhs` onto the end of `Self`.
///
/// This is the framework-internal operation that lets a tuple splice its
/// members' (self-contained, `Nil`-terminated) access lists together. The
/// `Nil` base case is where "continue on to the rest of the params" happens:
/// reaching the end of the left list grafts the entire right list on.
pub trait Concat<Rhs> {
    type Output;
}

impl<Rhs> Concat<Rhs> for Nil {
    type Output = Rhs;
}

impl<H, T, Rhs> Concat<Rhs> for Cons<H, T>
where
    T: Concat<Rhs>,
{
    type Output = Cons<H, <T as Concat<Rhs>>::Output>;
}

/// Builds a type-level access list terminated by [`Nil`].
///
/// ```ignore
/// accesses![Read<A>, Write<B>]  ==  Cons<Read<A>, Cons<Write<B>, Nil>>
/// accesses![]                   ==  Nil
/// ```
#[macro_export]
macro_rules! accesses {
    () => { $crate::Nil };
    ($head:ty $(, $tail:ty)* $(,)?) => {
        $crate::Cons<$head, $crate::accesses!($($tail),*)>
    };
}