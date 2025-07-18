use std::{
    collections::BTreeMap,
    fmt::{Debug, Formatter, Result},
    ops::Deref,
};

use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_map::{MutableBTreeMap, SignalMapExt},
    signal_vec::{MutableVec, SignalVec},
};
use futures_signals_ext::{MutableExt, MutableVecExt, SignalExtMapOption};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smol_str::{SmolStr, ToSmolStr, format_smolstr};

#[derive(Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    #[default]
    Error,
    Information,
    Section,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    message_type: MessageType,
    text: SmolStr,
    parameters: Vec<SmolStr>,
}

impl Message {
    fn new(message_type: MessageType, text: impl ToSmolStr) -> Self {
        Self {
            message_type,
            text: text.to_smolstr(),
            parameters: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_parameters(mut self, parameters: impl IntoIterator<Item = impl ToSmolStr>) -> Self {
        self.parameters = parameters
            .into_iter()
            .map(|item| item.to_smolstr())
            .collect();
        self
    }

    pub fn message_type(&self) -> MessageType {
        self.message_type
    }

    pub fn error(&self) -> bool {
        self.message_type == MessageType::Error
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn parameters(&self) -> &[SmolStr] {
        &self.parameters
    }

    pub fn localize<T>(&self, t: T) -> Self
    where
        T: Fn(&str) -> SmolStr,
    {
        let localized = t(self.text());
        let localized = if self.parameters().is_empty() {
            localized
        } else {
            let mut expanded = localized.to_string();
            for (index, parameter) in self.parameters().iter().enumerate() {
                expanded = expanded.replace(format_smolstr!("{{{index}}}").as_str(), parameter);
            }
            expanded.into()
        };
        Self {
            message_type: self.message_type,
            text: localized,
            parameters: vec![],
        }
    }
}

#[derive(Default, Clone)]
pub struct Messages {
    error: Mutable<bool>,
    messages: MutableBTreeMap<SmolStr, MutableVec<Message>>,
}

impl Serialize for Messages {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.messages.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Messages {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let messages = <MutableBTreeMap<SmolStr, MutableVec<Message>> as Deserialize>::deserialize(
            deserializer,
        )?;
        Ok(Self {
            error: Mutable::new(false),
            messages,
        })
    }
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
                f.write_str(match message.message_type {
                    MessageType::Error => "E: ",
                    MessageType::Information => "I: ",
                    MessageType::Section => "S: ",
                })?;
                f.write_str(message.text())?;
            }
            f.write_str("]")?;
        }
        Ok(())
    }
}

impl Deref for Messages {
    type Target = MutableBTreeMap<SmolStr, MutableVec<Message>>;

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

    pub fn extend(&self, with: Messages) {
        self.error.inspect_mut(|this| *this |= with.error.get());
        let mut this = self.lock_mut();
        let mut from = with.lock_mut();
        let from_keys = from.keys().cloned().collect::<Vec<_>>();
        for (key, messages) in from_keys.into_iter().map(|key| {
            let messages = from.remove(&key).unwrap_or_default();
            (key, messages)
        }) {
            if let Some(this_messages) = this.get(&key) {
                this_messages.extend_cloned(messages.lock_mut().drain(..));
            } else {
                this.insert_cloned(key, messages);
            }
        }
    }

    pub fn from_inner(inner: BTreeMap<SmolStr, MutableVec<Message>>) -> Self {
        Self {
            error: Mutable::new(false),
            messages: MutableBTreeMap::with_values(inner),
        }
    }

    pub fn into_inner(self) -> BTreeMap<SmolStr, MutableVec<Message>> {
        self.messages.lock_ref().deref().clone() // TODO: mem::replace? mem::take?
    }

    #[must_use]
    fn with(
        self,
        key: impl ToSmolStr,
        message_type: MessageType,
        text: impl ToSmolStr,
        parameters: impl IntoIterator<Item = impl ToSmolStr>,
    ) -> Self {
        self.add_with_pars(key, message_type, text, parameters);
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

    pub fn error_signal(&self) -> impl Signal<Item = bool> + use<> {
        self.error.signal().dedupe()
    }

    pub fn clear_all(&self) {
        self.messages.lock_mut().clear();
        self.error.set_neq(false);
    }

    pub fn set(&self, key: impl ToSmolStr, message_type: MessageType, message: impl ToSmolStr) {
        self.set_with_pars(key, message_type, message, [""; 0]);
    }

    pub fn set_with_pars(
        &self,
        key: impl ToSmolStr,
        message_type: MessageType,
        text: impl ToSmolStr,
        parameters: impl IntoIterator<Item = impl ToSmolStr>,
    ) {
        let message = Message::new(message_type, text).with_parameters(parameters);
        self.messages
            .lock_mut()
            .insert_cloned(key.to_smolstr(), MutableVec::new_with_values(vec![message]));
        self.error.set_neq(message_type == MessageType::Error);
    }

    pub fn add(&self, key: impl ToSmolStr, message_type: MessageType, text: impl ToSmolStr) {
        self.add_with_pars(key, message_type, text, [""; 0]);
    }

    pub fn add_with_pars(
        &self,
        key: impl ToSmolStr,
        message_type: MessageType,
        text: impl ToSmolStr,
        parameters: impl IntoIterator<Item = impl ToSmolStr>,
    ) {
        let key = key.to_smolstr();
        let message = Message::new(message_type, text).with_parameters(parameters);
        let mut lock = self.messages.lock_mut();
        if let Some(messages) = lock.get(&key) {
            messages.lock_mut().push_cloned(message);
        } else {
            lock.insert_cloned(key, MutableVec::new_with_values(vec![message]));
        }
        self.error
            .set_neq(self.error.get() || message_type == MessageType::Error);
    }

    pub fn clear(&self, key: impl ToSmolStr) {
        self.messages.lock_mut().remove(&key.to_smolstr());
        self.evaluate_error();
    }

    pub fn anything_for_key_signal<S: ToSmolStr>(
        &self,
        key: S,
    ) -> impl Signal<Item = bool> + use<S> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.to_smolstr())
            .map_some_default(|messages| !messages.lock_ref().is_empty())
    }

    pub fn error_for_key_signal<S: ToSmolStr>(&self, key: S) -> impl Signal<Item = bool> + use<S> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.to_smolstr())
            .map_some_default(|messages| messages.lock_ref().iter().any(Message::error))
    }

    pub fn messages_for_key_signal_vec<S: ToSmolStr>(
        &self,
        key: S,
    ) -> impl SignalVec<Item = Message> + use<S> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.to_smolstr())
            .switch_signal_vec(|messages| {
                messages
                    .map(|messages| messages.signal_vec_cloned())
                    .unwrap_or_else(|| MutableVec::new().signal_vec_cloned())
            })
    }

    pub fn add_entity_error(&self, message: impl ToSmolStr) {
        self.add(Self::ENTITY, MessageType::Error, message)
    }

    pub fn add_entity_info(&self, message: impl ToSmolStr) {
        self.add(Self::ENTITY, MessageType::Information, message)
    }

    pub fn add_service_error(&self, message: impl ToSmolStr) {
        self.add(Self::SERVICE, MessageType::Error, message)
    }

    pub fn add_service_info(&self, message: impl ToSmolStr) {
        self.add(Self::SERVICE, MessageType::Information, message)
    }

    pub fn from_service_error(message: impl ToSmolStr) -> Self {
        Self::new().with(Self::SERVICE, MessageType::Error, message, [""; 0])
    }

    pub fn from_service_error_with_pars(
        message: impl ToSmolStr,
        parameters: impl IntoIterator<Item = impl ToSmolStr>,
    ) -> Self {
        Self::new().with(Self::SERVICE, MessageType::Error, message, parameters)
    }

    pub fn from_entity_error(message: impl ToSmolStr) -> Self {
        Self::new().with(Self::ENTITY, MessageType::Error, message, [""; 0])
    }

    pub fn from_entity_error_with_pars(
        message: impl ToSmolStr,
        parameters: impl IntoIterator<Item = impl ToSmolStr>,
    ) -> Self {
        Self::new().with(Self::ENTITY, MessageType::Error, message, parameters)
    }

    pub fn localize<T>(self, t: T) -> Self
    where
        T: Fn(&str) -> SmolStr,
    {
        let localized = self
            .messages
            .lock_ref()
            .iter()
            .map(|(key, messages)| {
                let localized = messages
                    .lock_ref()
                    .iter()
                    .map(|message| message.localize(&t))
                    .collect();
                (key.clone(), MutableVec::new_with_values(localized))
            })
            .collect();

        Self {
            error: self.error,
            messages: MutableBTreeMap::with_values(localized),
        }
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
