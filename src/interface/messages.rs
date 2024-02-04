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
use futures_signals_ext::SignalExtMapOption;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    error: bool,
    text: SmolStr,
    parameters: Vec<SmolStr>,
}

impl Message {
    fn new(error: bool, text: impl Into<SmolStr>) -> Self {
        Self {
            error,
            text: text.into(),
            parameters: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_parameters(
        mut self,
        parameters: impl IntoIterator<Item = impl Into<SmolStr>>,
    ) -> Self {
        self.parameters = parameters.into_iter().map(Into::into).collect();
        self
    }

    pub fn error(&self) -> bool {
        self.error
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn parameters(&self) -> &[SmolStr] {
        &self.parameters
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Messages {
    #[serde(skip)]
    error: Mutable<bool>,
    messages: MutableBTreeMap<SmolStr, MutableVec<Message>>,
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
    type Target = MutableBTreeMap<SmolStr, MutableVec<Message>>;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

impl Messages {
    pub const SERVICE: &'static str = "service";
    pub const ENTITY: &'static str = "entity";

    pub const NONE_PARS: Option<[&'static str; 0]> = None;

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
        key: impl Into<SmolStr>,
        error: bool,
        text: impl Into<SmolStr>,
        parameters: Option<impl IntoIterator<Item = impl Into<SmolStr>>>,
    ) -> Self {
        self.add(key, error, text, parameters);
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

    pub fn set<K, M>(
        &self,
        key: impl Into<SmolStr>,
        error: bool,
        message: impl Into<SmolStr>,
        parameters: Option<impl IntoIterator<Item = impl Into<SmolStr>>>,
    ) {
        let message = if let Some(parameters) = parameters {
            Message::new(error, message).with_parameters(parameters)
        } else {
            Message::new(error, message)
        };
        self.messages
            .lock_mut()
            .insert_cloned(key.into(), MutableVec::new_with_values(vec![message]));
        self.error.set_neq(error);
    }

    pub fn add(
        &self,
        key: impl Into<SmolStr>,
        error: bool,
        text: impl Into<SmolStr>,
        parameters: Option<impl IntoIterator<Item = impl Into<SmolStr>>>,
    ) {
        let key = key.into();
        let message = if let Some(parameters) = parameters {
            Message::new(error, text).with_parameters(parameters)
        } else {
            Message::new(error, text)
        };
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
        K: Into<SmolStr>,
    {
        self.messages.lock_mut().remove(&key.into());
        self.evaluate_error();
    }

    pub fn anything_for_key_signal(&self, key: impl Into<SmolStr>) -> impl Signal<Item = bool> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.into())
            .map_some_default(|messages| !messages.lock_ref().is_empty())
    }

    pub fn error_for_key_signal(&self, key: impl Into<SmolStr>) -> impl Signal<Item = bool> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.into())
            .map_some_default(|messages| messages.lock_ref().iter().any(Message::error))
    }

    pub fn messages_for_key_signal_vec(
        &self,
        key: impl Into<SmolStr>,
    ) -> impl SignalVec<Item = Message> {
        self.messages
            .signal_map_cloned()
            .key_cloned(key.into())
            .switch_signal_vec(|messages| {
                messages
                    .map(|messages| messages.signal_vec_cloned())
                    .unwrap_or_else(|| MutableVec::new().signal_vec_cloned())
            })
    }

    pub fn add_entity_error(&self, message: impl Into<SmolStr>) {
        self.add(Self::ENTITY, true, message, Self::NONE_PARS)
    }

    pub fn add_entity_info(&self, message: impl Into<SmolStr>) {
        self.add(Self::ENTITY, false, message, Self::NONE_PARS)
    }

    pub fn add_service_error(&self, message: impl Into<SmolStr>) {
        self.add(Self::SERVICE, true, message, Self::NONE_PARS)
    }

    pub fn add_service_info(&self, message: impl Into<SmolStr>) {
        self.add(Self::SERVICE, false, message, Self::NONE_PARS)
    }

    pub fn from_service_error(message: impl Into<SmolStr>) -> Self {
        Self::new().with(Self::SERVICE, true, message, Self::NONE_PARS)
    }

    pub fn from_service_error_pars(
        message: impl Into<SmolStr>,
        parameters: impl IntoIterator<Item = impl Into<SmolStr>>,
    ) -> Self {
        Self::new().with(Self::SERVICE, true, message, Some(parameters))
    }

    pub fn from_entity_error(message: impl Into<SmolStr>) -> Self {
        Self::new().with(Self::ENTITY, true, message, Self::NONE_PARS)
    }

    pub fn from_entity_error_pars(
        message: impl Into<SmolStr>,
        parameters: impl IntoIterator<Item = impl Into<SmolStr>>,
    ) -> Self {
        Self::new().with(Self::ENTITY, true, message, Some(parameters))
    }

    pub fn localize<T>(self, locale: &str, t: T) -> Self
    where
        T: Fn(&str, &str) -> SmolStr,
    {
        let localized = self
            .messages
            .lock_ref() 
            .iter()
            .map(|(key, messages)| {
                let localized = messages
                    .lock_ref()
                    .iter()
                    .map(|message| {
                        Message::new(message.error(), {
                            let mut localized = t(locale, message.text());
                            if !message.parameters().is_empty() {
                                let mut expanded = localized.to_string();
                                for (index, parameter) in message.parameters().iter().enumerate() {
                                    expanded = expanded.replace(&format!("{index}"), parameter);
                                }
                                localized = expanded.into();
                            }
                            localized
                        })
                    })
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
