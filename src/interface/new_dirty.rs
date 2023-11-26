use std::ops::{Deref, DerefMut};

pub trait Inner<E>
where
    Self: Deref<Target = E> + DerefMut,
{
    #[must_use]
    fn from_inner(inner: E) -> Self;

    #[must_use]
    fn into_inner(self) -> E
    where
        Self: Sized;

    fn inner(&self) -> &E {
        self.deref()
    }
}

pub trait New {
    fn is_new(&self) -> bool;

    #[must_use]
    fn with_new(self) -> Self
    where
        Self: Sized;

    #[must_use]
    fn with_existing(self) -> Self
    where
        Self: Sized;
}

pub trait Dirty {
    fn is_dirty(&self) -> bool;

    fn take_dirty(&mut self) -> bool;

    #[must_use]
    fn with_dirty(self) -> Self
    where
        Self: Sized;

    fn mark_as_dirty(&mut self);
}
