use std::ops::{Deref, DerefMut};

pub trait Inner<E>
where
    Self: Sized + Deref<Target = E> + DerefMut,
{
    #[must_use]
    fn from_inner(inner: E) -> Self;

    #[must_use]
    fn into_inner(self) -> E;

    fn inner(&self) -> &E {
        self.deref()
    }
}

pub trait New
where
    Self: Sized,
{
    fn is_new(&self) -> bool {
        false
    }

    #[must_use]
    fn with_new(self) -> Self {
        self
    }

    #[must_use]
    fn with_existing(self) -> Self {
        self
    }
}

pub trait Dirty
where
    Self: Sized,
{
    fn is_dirty(&self) -> bool {
        false
    }

    #[must_use]
    fn with_dirty(self) -> Self {
        self
    }

    fn mark_as_dirty(&mut self) {}
}
