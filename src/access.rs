use core::marker::PhantomData;

// Access kinds
pub struct Read<T>(PhantomData<T>);
pub struct Write<T>(PhantomData<T>);
