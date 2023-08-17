use std::{
    fmt::{Debug, Formatter, Result},
    ops::Deref,
};

use futures_signals::{signal_map::MutableBTreeMap, signal_vec::MutableVec};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    error: bool,
    text: String,
}

impl Message {
    pub fn new(error: bool, text: impl ToString) -> Self {
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
pub struct Messages(MutableBTreeMap<String, MutableVec<Message>>);

impl From<&str> for Messages {
    fn from(message: &str) -> Self {
        Messages::from_service_error(message)
    }
}

impl Debug for Messages {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let formatted = self
            .0
            .iter()
            .fold(String::new(), |mut output, (key, item)| {
                if let Some(ref error) = item.error {
                    output = output + key + ".errors: [\n  " + &error.join(",\n  ") + "\n]\n";
                }
                if let Some(ref info) = item.info {
                    output = output + key + ".infos: [\n  " + &info.join(",\n  ") + "\n]\n";
                }
                output
            });
        f.write_str(&formatted)
    }
}

impl Deref for Messages {
    type Target = MutableBTreeMap<String, MutableVec<Message>>;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

impl Messages {
    pub const SERVICE: &'static str = "service";
    pub const ENTITY: &'static str = "entity";

    pub fn new() -> Messages {
        Self(MutableBTreeMap::new())
    }

    pub fn replace(&self, with: Messages) {
        self.lock_mut().clone_from(&with.lock_ref());
    }

    #[must_use]
    fn with<K, M>(mut self, key: K, error: bool, message: M) -> Self
    where
        K: ToString,
        M: ToString,
    {
        self.add(key, error, message);
        self
    }

    pub fn error(&self) -> bool {
        self.0
            .lock_ref()
            .values()
            .any(|value| value.lock_ref().iter().any(Message::error))
    }

    pub fn clear_all(&self) {
        self.0.lock_mut().clear();
    }

    pub fn set<K, M>(&self, key: K, error: bool, message: M)
    where
        K: ToString,
        M: ToString,
    {
        self.0.lock_mut().insert(
            key.to_string(),
            MutableVec::new_with_values(vec![Message::new(error, message.to_string())]),
        );
    }

    pub fn add<K, M>(&self, key: K, error: bool, message: M)
    where
        K: ToString,
        M: ToString,
    {
        self.0
            .lock_mut()
            .entry(key.to_string())
            .or_insert(MutableVec::new())
            .lock_mut()
            .push_cloned(Message::new(error, message.to_string()));
    }

    pub fn clear<K>(&self, key: K)
    where
        K: AsRef<str>,
    {
        self.0.lock_mut().remove_entry(key.as_ref());
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
        assert_eq!("entity.errors: [\n  EE\n]\n", format!("{messages:?}"));
    }

    #[test]
    fn object_is_created_from_service_error() {
        let messages = Messages::from_service_error("SE");
        assert!(messages.error());
        assert_eq!("service.errors: [\n  SE\n]\n", format!("{messages:?}"));
    }

    #[test]
    fn add_service_info_works() {
        let mut messages = Messages::from_entity_error("EE");
        messages.add_service_info("SI");
        let output = format!("{messages:?}");
        assert!(
            "service.infos: [\n  SI\n]\nentity.errors: [\n  EE\n]\n" == output
                || "entity.errors: [\n  EE\n]\nservice.infos: [\n  SI\n]\n" == output
        );
    }

    #[test]
    fn add_service_error_works() {
        let mut messages = Messages::from_service_error("SE");
        messages.add_service_error("SE");
        let output = format!("{messages:?}");
        assert_eq!("service.errors: [\n  SE,\n  SE\n]\n", output);
    }

    #[test]
    fn add_entity_info_works() {
        let mut messages = Messages::from_entity_error("EE");
        messages.add_entity_info("EI");
        let output = format!("{messages:?}");
        assert!(
            "entity.infos: [\n  EI\n]\nentity.errors: [\n  EE\n]\n" == output
                || "entity.errors: [\n  EE\n]\nentity.infos: [\n  EI\n]\n" == output
        );
    }

    #[test]
    fn add_entity_error_works() {
        let mut messages = Messages::from_entity_error("EE");
        messages.add_entity_error("EE");
        let output = format!("{messages:?}");
        assert_eq!("entity.errors: [\n  EE,\n  EE\n]\n", output);
    }
}
