use serde::{Deserialize, Serialize};
#[cfg(not(feature = "postcard"))]
use serde_with::skip_serializing_none;

use crate::Messages;

#[cfg_attr(not(feature = "postcard"), skip_serializing_none)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EntityResponse<E> {
    messages: Messages,
    entity: Option<E>,
}

impl<E> EntityResponse<E> {
    pub fn new(messages: Messages) -> Self {
        Self {
            messages,
            entity: None,
        }
    }

    #[must_use]
    pub fn with_entity(mut self, entity: E) -> Self {
        self.entity = Some(entity);
        self
    }

    pub fn messages(&self) -> &Messages {
        &self.messages
    }

    pub fn entity(&self) -> Option<&E> {
        self.entity.as_ref()
    }

    pub fn take(self) -> (Option<E>, Messages) {
        (self.entity, self.messages)
    }
}

#[cfg_attr(not(feature = "postcard"), skip_serializing_none)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CollectionResponse<E> {
    messages: Messages,
    paging: Paging,
    collection: Option<Vec<E>>,
}

impl<E> CollectionResponse<E> {
    pub fn new(messages: Messages) -> Self {
        Self {
            messages,
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
        (self.collection, self.messages, self.paging)
    }
}

#[cfg_attr(not(feature = "postcard"), skip_serializing_none)]
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
