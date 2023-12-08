use std::collections::BTreeMap;

use futures_signals::signal_vec::MutableVec;
use serde::{Deserialize, Serialize};
#[cfg(all(feature = "json", not(feature = "postcard")))]
use serde_with::skip_serializing_none;
use smol_str::SmolStr;

use crate::{Message, Messages};

#[cfg_attr(
    all(feature = "json", not(feature = "postcard")),
    skip_serializing_none
)]
#[derive(Default, Serialize, Deserialize)]
pub struct EntityResponse<E> {
    messages: BTreeMap<SmolStr, MutableVec<Message>>,
    entity: Option<E>,
}

impl<E> EntityResponse<E> {
    pub fn new(messages: Messages) -> Self {
        Self {
            messages: messages.into_inner(),
            entity: None,
        }
    }

    #[must_use]
    pub fn with_entity(mut self, entity: E) -> Self {
        self.entity = Some(entity);
        self
    }

    pub fn take(self) -> (Option<E>, Messages) {
        (self.entity, Messages::from_inner(self.messages))
    }
}

#[cfg_attr(
    all(feature = "json", not(feature = "postcard")),
    skip_serializing_none
)]
#[derive(Default, Serialize, Deserialize)]
pub struct CollectionResponse<E> {
    messages: BTreeMap<SmolStr, MutableVec<Message>>,
    paging: Paging,
    collection: Option<Vec<E>>,
}

impl<E> CollectionResponse<E> {
    pub fn new(messages: Messages) -> Self {
        Self {
            messages: messages.into_inner(),
            paging: Paging::default(),
            collection: None,
        }
    }

    #[must_use]
    pub fn with_collection(mut self, collection: Vec<E>) -> Self {
        self.collection = Some(collection);
        self
    }

    #[must_use]
    pub fn with_paging(mut self, paging: Paging) -> Self {
        self.paging = paging;
        self
    }

    pub fn take(self) -> (Option<Vec<E>>, Messages, Paging) {
        (
            self.collection,
            Messages::from_inner(self.messages),
            self.paging,
        )
    }
}

#[cfg_attr(
    all(feature = "json", not(feature = "postcard")),
    skip_serializing_none
)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Paging {
    limit: usize,
    prev: Option<String>,
    next: Option<String>,
}

impl Default for Paging {
    fn default() -> Self {
        Self {
            limit: 25,
            prev: None,
            next: None,
        }
    }
}
