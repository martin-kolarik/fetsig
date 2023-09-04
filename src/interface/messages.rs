use std::{
    fmt::{Debug, Formatter, Result},
    ops::Deref,
};

use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_map::{MutableBTreeMap, SignalMapExt},
    signal_vec::{MutableSignalVec, MutableVec},
};
use futures_signals_ext::SignalExtMapOption;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    error: bool,
    text: String,
}

impl Message {
    fn new(error: bool, text: impl ToString) -> Self {
        Self {
            error,
            text: text.to_string(),
        }
    }

    pub fn error(&self) -> bool {
        self.error
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Messages {
    #[serde(skip)]
    error: Mutable<bool>,
    messages: MutableBTreeMap<String, MutableVec<Message>>,
}

impl From<&str> for Messages {
    fn from(message: &str) -> Self {
        Messages::from_service_error(message)
    }
}

impl Debug for Messages {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for (index, (key, messages)) in self.messages.lock_ref().iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            f.write_str(key)?;
            f.write_str(": [")?;
            for (index, message) in messages.lock_ref().iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                f.write_str(if message.error { "E: " } else { "I: " })?;
                f.write_str(message.text())?;
            }
            f.write_str("]")?;
        }
        Ok(())
    }
}

impl Deref for Messages {
    type Target = MutableBTreeMap<String, MutableVec<Message>>;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

impl Messages {
    pub const SERVICE: &'static str = "service";
    pub const ENTITY: &'static str = "entity";

    pub fn new() -> Messages {
        Self {
            error: Mutable::new(false),
            messages: MutableBTreeMap::new(),
        }
    }

    pub fn replace(&self, with: Messages) {
        self.lock_mut().replace_cloned(with.lock_mut().clone());
        self.evaluate_error();
    }

    #[must_use]
    fn with<K, M>(self, key: K, error: bool, message: M) -> Self
    where
        K: ToString,
        M: ToString,
    {
        self.add(key, error, message);
        self
    }

    pub fn error(&self) -> bool {
        self.error.get()
    }

    fn evaluate_error(&self) {
        let error = self
            .messages
            .lock_ref()
            .values()
            .any(|messages| messages.lock_ref().iter().any(Message::error));
        self.error.set_neq(error);
    }

    pub fn error_signal(&self) -> impl Signal<Item = bool> {
        self.error.signal().dedupe()
    }

    pub fn clear_all(&self) {
        self.messages.lock_mut().clear();
        self.error.set_neq(false);
    }

    pub fn set<K, M>(&self, key: K, error: bool, message: M)
    where
        K: ToString,
        M: ToString,
    {
        self.messages.lock_mut().insert_cloned(
            key.to_string(),
            MutableVec::new_with_values(vec![Message::new(error, message.to_string())]),
        );
        self.error.set_neq(error);
    }

    pub fn add<K, M>(&self, key: K, error: bool, message: M)
    where
        K: ToString,
        M: ToString,
    {
        let key = key.to_string();
        let message = Message::new(error, message.to_string());
        let mut lock = self.messages.lock_mut();
        if let Some(messages) = lock.get(&key) {
            messages.lock_mut().push_cloned(message);
        } else {
            lock.insert_cloned(key, MutableVec::new_with_values(vec![message]));
        }
        self.error.set_neq(self.error.get() || error);
    }

    pub fn clear<K>(&self, key: K)
    where
        K: ToString,
    {
        self.messages.lock_mut().remove(&key.to_string());
        self.evaluate_error();
    }

    pub fn error_for_key_signal(&self, key: impl ToString) -> impl Signal<Item = bool> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.to_string())
            .map_some_default(|messages| messages.lock_ref().iter().any(Message::error))
    }

    pub fn messages_for_key_signal_vec(
        &self,
        key: impl ToString,
    ) -> impl Signal<Item = MutableSignalVec<Message>> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.to_string())
            .map(|messages| {
                messages
                    .map(|messages| messages.signal_vec_cloned())
                    .unwrap_or_else(|| MutableVec::new().signal_vec_cloned())
            })
    }

    pub fn add_entity_error(&self, message: impl ToString) {
        self.add(Self::ENTITY, true, message)
    }

    pub fn add_entity_info(&self, message: impl ToString) {
        self.add(Self::ENTITY, false, message)
    }

    pub fn add_service_error(&self, message: impl ToString) {
        self.add(Self::SERVICE, true, message)
    }

    pub fn add_service_info(&self, message: impl ToString) {
        self.add(Self::SERVICE, false, message)
    }

    pub fn from_service_error(message: impl ToString) -> Self {
        Self::new().with(Messages::SERVICE, true, message)
    }

    pub fn from_entity_error(message: impl ToString) -> Self {
        Self::new().with(Messages::ENTITY, true, message)
    }
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]

mod tests {
    use super::*;

    #[test]
    fn object_is_created() {
        Messages::new();
        assert!(true);
    }

    #[test]
    fn object_is_converted_from_str() {
        let messages: Messages = "XX".into();
        assert!(messages.error());
        assert!(true);
    }

    #[test]
    fn object_is_created_from_entity_error() {
        let messages = Messages::from_entity_error("EE");
        assert!(messages.error());
        assert_eq!("entity: [E: EE]", format!("{messages:?}"));
    }

    #[test]
    fn object_is_created_from_service_error() {
        let messages = Messages::from_service_error("SE");
        assert!(messages.error());
        assert_eq!("service: [E: SE]", format!("{messages:?}"));
    }

    #[test]
    fn add_service_info_works() {
        let messages = Messages::from_entity_error("EE");
        messages.add_service_info("SI");
        let output = format!("{messages:?}");
        assert_eq!("entity: [E: EE], service: [I: SI]", output);
    }

    #[test]
    fn add_service_error_works() {
        let messages = Messages::from_service_error("SE");
        messages.add_service_error("SE");
        let output = format!("{messages:?}");
        assert_eq!("service: [E: SE, E: SE]", output);
    }

    #[test]
    fn add_entity_info_works() {
        let messages = Messages::from_entity_error("EE");
        messages.add_entity_info("EI");
        let output = format!("{messages:?}");
        assert_eq!("entity: [E: EE, I: EI]", output);
    }

    #[test]
    fn add_entity_error_works() {
        let messages = Messages::from_entity_error("EE");
        messages.add_entity_error("EE");
        let output = format!("{messages:?}");
        assert_eq!("entity: [E: EE, E: EE]", output);
    }
}
