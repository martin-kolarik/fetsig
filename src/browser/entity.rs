use std::marker::PhantomData;

use futures_signals::signal::{and, not, Mutable, MutableLockMut, MutableLockRef, Signal};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen_futures::spawn_local;

use crate::{MacSign, MacVerify, MediaType, NoMac, StatusCode, HEADER_SIGNATURE};

use super::{
    common::{execute_fetch, PendingFetch},
    request::Request,
    transferstate::TransferState,
};

#[derive(Debug)]
pub struct EntityStore<E, MV = NoMac> {
    transfer_state: Mutable<TransferState>,
    messages: Mutable<Messages>,
    entity: MutableOption<E>,
    pmv: PhantomData<MV>,
}

impl<E, MV> EntityStore<E, MV>
where
    E: Clone + Serialize + DeserializeOwned,
    MV: MacVerify,
{
    pub fn new_empty() -> Self {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
            messages: Mutable::new(Messages::default()),
            entity: MutableOption::new_empty(),
            pmv: PhantomData,
        }
    }

    pub fn new_default() -> Self
    where
        E: Default,
    {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
            messages: Mutable::new(Messages::default()),
            entity: MutableOption::new_default(),
            pmv: PhantomData,
        }
    }

    pub fn new_value(entity: E) -> Self {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
            messages: Mutable::new(Messages::default()),
            entity: MutableOption::new_some_value(entity),
            pmv: PhantomData,
        }
    }

    pub fn reset_to_empty(&self) {
        self.transfer_state.set(TransferState::Empty);
        self.messages.set(Messages::default());
        self.reset();
    }

    pub fn reset_to_default(&self)
    where
        E: Default,
    {
        self.transfer_state.set(TransferState::Empty);
        self.messages.set(Messages::default());
        self.set(Some(E::default()));
    }

    pub fn reset_to_value(&self, entity: E) {
        self.transfer_state.set(TransferState::Empty);
        self.messages.set(Messages::default());
        self.set(Some(entity));
    }

    pub fn reset_to_inner<I>(&self, entity: I)
    where
        E: Inner<I>,
    {
        self.transfer_state.set(TransferState::Empty);
        self.messages.set(Messages::default());
        self.set(Some(E::from_inner(entity)));
    }

    pub fn empty(&self) -> bool {
        self.entity.lock_ref().is_none()
    }

    pub fn not_empty(&self) -> bool {
        self.entity.lock_ref().is_some()
    }

    pub fn empty_signal(&self) -> impl Signal<Item = bool> {
        self.entity.signal_ref(Option::is_none).dedupe()
    }

    pub fn not_empty_signal(&self) -> impl Signal<Item = bool> {
        self.entity.signal_ref(Option::is_some).dedupe()
    }

    pub fn invalidate(&self) {
        self.transfer_state.set(TransferState::Empty);
    }

    pub fn transfer_state(&self) -> &Mutable<TransferState> {
        &self.transfer_state
    }

    pub fn set_transfer_state(&self, transfer_state: TransferState) {
        self.transfer_state.set_neq(transfer_state);
    }

    pub fn reset_transfer_error(&self) {
        self.transfer_state.lock_mut().reset_error();
    }

    pub fn loaded(&self) -> bool {
        self.transfer_state.map(TransferState::loaded)
    }

    pub fn loaded_signal(&self) -> impl Signal<Item = bool> {
        self.transfer_state
            .signal_ref(TransferState::loaded)
            .dedupe()
    }

    pub fn loaded_status(&self) -> StatusCode {
        self.transfer_state.map(TransferState::loaded_status)
    }

    pub fn loaded_status_signal(&self) -> impl Signal<Item = StatusCode> {
        self.transfer_state
            .signal_ref(TransferState::loaded_status)
            .dedupe()
    }

    pub fn stored(&self) -> bool {
        self.transfer_state.map(TransferState::stored)
    }

    pub fn stored_signal(&self) -> impl Signal<Item = bool> {
        self.transfer_state
            .signal_ref(TransferState::stored)
            .dedupe()
    }

    pub fn stored_status(&self) -> StatusCode {
        self.transfer_state.map(TransferState::stored_status)
    }

    pub fn stored_status_signal(&self) -> impl Signal<Item = StatusCode> {
        self.transfer_state
            .signal_ref(TransferState::stored_status)
            .dedupe()
    }

    pub fn pending(&self) -> bool {
        self.transfer_state.map(TransferState::pending)
    }

    pub fn pending_signal(&self) -> impl Signal<Item = bool> {
        self.transfer_state
            .signal_ref(TransferState::pending)
            .dedupe()
    }

    pub fn dirty_signal(&self) -> impl Signal<Item = bool>
    where
        E: Dirty,
    {
        self.entity
            .signal_ref(|e| e.as_ref().map(|e| e.is_dirty()).unwrap_or(false))
            .dedupe()
    }

    pub fn messages_error_signal(&self) -> impl Signal<Item = bool> {
        self.messages.signal_ref(Messages::is_error).dedupe()
    }

    pub fn can_commit_signal(&self) -> impl Signal<Item = bool>
    where
        E: Dirty,
    {
        and(self.dirty_signal(), not(self.messages_error_signal())).dedupe()
    }

    pub fn messages(&self) -> &Mutable<Messages> {
        &self.messages
    }

    pub fn entity(&self) -> &MutableOption<E> {
        &self.entity
    }

    #[inline]
    pub fn signal_map<F, U>(&self, f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(Option<&E>) -> Option<U>,
    {
        self.entity.signal_map(f)
    }

    #[inline]
    pub fn signal_map_some<F, U>(&self, f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&E) -> U,
    {
        self.entity.signal_map_some(f)
    }

    #[inline]
    pub fn signal_map_some_or<F, U>(&self, f: F, default: U) -> impl Signal<Item = U>
    where
        F: FnMut(&E) -> U,
        U: Clone,
    {
        self.entity.signal_map_some_or(f, default)
    }

    #[inline]
    pub fn signal_map_some_or_else<F, D, U>(&self, f: F, default: D) -> impl Signal<Item = U>
    where
        F: FnMut(&E) -> U,
        D: FnOnce() -> U + Clone,
    {
        self.entity.signal_map_some_or_else(f, default)
    }

    #[inline]
    pub fn signal_and_then_some<F, U>(&self, f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&E) -> Option<U>,
    {
        self.entity.signal_and_then_some(f)
    }

    #[inline]
    pub fn signal_and_then_some_or<F, U>(&self, f: F, default: U) -> impl Signal<Item = U>
    where
        F: FnMut(&E) -> Option<U>,
        U: Clone,
    {
        self.entity.signal_and_then_some_or(f, default)
    }

    #[inline]
    pub fn signal_and_then_some_or_else<F, D, U>(&self, f: F, default: D) -> impl Signal<Item = U>
    where
        F: FnMut(&E) -> Option<U>,
        D: FnOnce() -> U + Clone,
    {
        self.entity.signal_and_then_some_or_else(f, default)
    }

    #[inline]
    pub fn signal_map_some_default<F, U>(&self, f: F) -> impl Signal<Item = U>
    where
        F: FnMut(&E) -> U,
        U: Default,
    {
        self.entity.signal_map_some_default(f)
    }

    pub fn lock_ref(&self) -> MutableLockRef<Option<E>> {
        self.entity.lock_ref()
    }

    pub fn lock_mut(&self) -> MutableLockMut<Option<E>> {
        self.entity.lock_mut()
    }

    pub fn inspect<F>(&self, f: F)
    where
        F: FnOnce(&E),
    {
        let _ = self.entity.lock_ref().as_ref().map(f);
    }

    pub fn inspect_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut E),
    {
        self.entity.lock_mut().as_mut().map(f);
    }

    pub fn inspect_mut_map<F, U>(&self, f: F) -> Option<U>
    where
        F: FnOnce(&mut E) -> U,
    {
        self.entity.lock_mut().as_mut().map(f)
    }

    pub fn map<F, U>(&self, f: F) -> Option<U>
    where
        F: FnOnce(&E) -> U,
    {
        self.entity.lock_ref().as_ref().map(f)
    }

    pub fn map_or_default<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&E) -> U,
        U: Default,
    {
        self.entity.lock_ref().as_ref().map(f).unwrap_or_default()
    }

    pub fn and_then<F, U>(&self, f: F) -> Option<U>
    where
        F: FnOnce(&E) -> Option<U>,
    {
        self.entity.lock_ref().as_ref().and_then(f)
    }

    pub fn get(&self) -> Option<E> {
        self.entity.get_cloned()
    }

    pub fn set(&self, entity: Option<E>) {
        self.entity.set(entity)
    }

    pub fn set_neq(&self, entity: Option<E>)
    where
        E: PartialEq,
    {
        self.entity.set_neq(entity);
    }

    pub fn set_externally_loaded(&self, entity: Option<E>) {
        self.entity.set(entity);
        self.set_transfer_state(TransferState::Loaded(StatusCode::Ok));
    }

    pub fn set_inner<I>(&self, entity: Option<I>)
    where
        E: Inner<I>,
    {
        self.set(entity.map(E::from_inner));
    }

    pub fn set_inner_neq<I>(&self, entity: Option<I>)
    where
        E: PartialEq + Inner<I>,
    {
        self.set_neq(entity.map(E::from_inner));
    }

    pub fn set_externally_loaded_inner<I>(&self, entity: Option<I>)
    where
        E: PartialEq + Inner<I>,
    {
        self.set_externally_loaded(entity.map(E::from_inner));
    }

    pub fn reset(&self) {
        self.entity.set(None);
    }

    pub fn replace(&self, entity: Option<E>) -> Option<E> {
        self.entity.replace(entity)
    }

    pub fn load<C>(&self, request: Request<'_>, result_callback: C)
    where
        C: FnOnce(StatusCode) + 'static,
    {
        if self.transfer_state.map(TransferState::loaded) {
            if request.logging() {
                log::debug!("Request to load {} skipped, using cache", request.url());

                if !request.method().is_load() {
                    log::warn!(
                        "Load request unexpectedly uses store verb {:?}",
                        request.method().as_str()
                    );
                }
            }
        } else {
            self.load_skip_cache(request, result_callback);
        }
    }

    pub fn load_skip_cache<C>(&self, request: Request<'_>, result_callback: C)
    where
        C: FnOnce(StatusCode) + 'static,
    {
        if request.logging() {
            log::debug!("Request to load {}", request.url());

            if !request.method().is_load() {
                log::warn!(
                    "Load request unexpectedly uses store verb {:?}",
                    request.method().as_str()
                );
            }
        }

        fetch::<_, _, MV>(
            request.with_is_load(true),
            self.transfer_state.clone(),
            self.messages.clone(),
            Some(self.entity.clone()),
            result_callback,
        );
    }

    pub fn load_with_request<MS, R, C>(
        &self,
        request: Request<'_>,
        request_entity: MutableOption<R>,
        result_callback: C,
    ) where
        MS: MacSign,
        R: Clone + Serialize + DeserializeOwned,
        C: FnOnce(StatusCode) + 'static,
    {
        store::<_, _, _, MS, MV>(
            request.with_is_load(true),
            self.transfer_state.clone(),
            self.messages.clone(),
            request_entity,
            Some(self.entity.clone()),
            result_callback,
        );
    }

    pub fn execute<C>(&self, request: Request<'_>, result_callback: C)
    where
        C: FnOnce(StatusCode) + 'static,
    {
        if request.logging() {
            log::debug!("Request to execute {}", request.url());

            if request.method().is_load() {
                log::warn!(
                    "Execute request unexpectedly uses load verb {:?}",
                    request.method().as_str()
                );
            }
        }

        fetch::<String, _, MV>(
            request.with_is_load(false),
            self.transfer_state.clone(),
            self.messages.clone(),
            None,
            result_callback,
        );
    }

    pub fn execute_with_response<R, C>(
        &self,
        request: Request<'_>,
        response_entity: MutableOption<R>,
        result_callback: C,
    ) where
        R: Clone + Serialize + DeserializeOwned,
        C: FnOnce(StatusCode) + 'static,
    {
        if request.logging() {
            log::debug!("Request to execute {}", request.url());

            if request.method().is_load() {
                log::warn!(
                    "Execute request unexpectedly uses load verb {:?}",
                    request.method().as_str()
                );
            }

            if !request.wants_response() {
                log::warn!("Execute expects response, but request does not",);
            }
        }

        fetch::<_, _, MV>(
            request.with_is_load(false),
            self.transfer_state.clone(),
            self.messages.clone(),
            Some(response_entity),
            result_callback,
        );
    }

    pub fn store<MS, C>(&self, request: Request<'_>, result_callback: C)
    where
        MS: MacSign,
        C: FnOnce(StatusCode) + 'static,
    {
        let response_entity = if request.wants_response() {
            Some(self.entity.clone())
        } else {
            None
        };
        store::<_, _, _, MS, MV>(
            request.with_is_load(false),
            self.transfer_state.clone(),
            self.messages.clone(),
            self.entity.clone(),
            response_entity,
            result_callback,
        )
    }

    pub fn store_with_response<MS, R, C>(
        &self,
        request: Request<'_>,
        response_entity: MutableOption<R>,
        result_callback: C,
    ) where
        MS: MacSign,
        R: Clone + Serialize + DeserializeOwned,
        C: FnOnce(StatusCode) + 'static,
    {
        store::<_, _, _, MS, MV>(
            request.with_is_load(false),
            self.transfer_state.clone(),
            self.messages.clone(),
            self.entity.clone(),
            Some(response_entity),
            result_callback,
        );
    }
}

fn store<E, R, C, MS, MV>(
    mut request: Request<'_>,
    transfer_state: Mutable<TransferState>,
    messages: Mutable<Messages>,
    request_entity: MutableOption<E>,
    storage_entity: Option<MutableOption<R>>,
    result_callback: C,
) where
    E: Serialize,
    R: DeserializeOwned + 'static,
    C: FnOnce(StatusCode) + 'static,
    MS: MacSign,
    MV: MacVerify,
{
    if request.logging() {
        log::debug!("Request to store {}", request.url());

        if request.method().is_load() {
            log::warn!(
                "Store request unexpectedly uses load verb {:?}",
                request.method().as_str()
            );
        }

        if storage_entity.is_none() && request.wants_response() {
            log::warn!("Store request wants response but defines no response entity",);
        }
    }

    let media_type = match request.media_type() {
        Some(media_type @ MediaType::Json | media_type @ MediaType::Postcard) => media_type,
        _ => {
            if request.logging() {
                log::warn!("Request failed as unsupported media type is requested");
            }
            *messages.lock_mut() = Messages::from_service_error(
                "Request failed as unsupported media type is requested",
            );
            transfer_state
                .lock_mut()
                .stop(StatusCode::UnsupportedMediaType);
            return;
        }
    };

    {
        // scope around content borrow
        let content = request_entity.lock_ref();
        let bytes = match (&*content, media_type) {
            (None, _) => {
                if request.logging() {
                    log::error!("Cannot store nonexisting entity, unexpected code flow");
                }
                return;
            }
            (Some(content), MediaType::Json) => content.to_json(),
            (Some(content), MediaType::Postcard) => content.to_postcard(),
            _ => {
                if request.logging() {
                    log::error!("Unsupported media type requested, unexpected code flow");
                }
                return;
            }
        };

        if let Some(signature) = MS::sign(bytes.as_ref()) {
            request = request.with_header(HEADER_SIGNATURE, signature);
        }

        request = request.with_body(bytes);
    }

    fetch::<_, _, MV>(
        request,
        transfer_state,
        messages,
        storage_entity,
        result_callback,
    );
}

pub(super) fn fetch<R, C, MV>(
    request: Request<'_>,
    transfer_state: Mutable<TransferState>,
    messages: Mutable<Messages>,
    storage_entity: Option<MutableOption<R>>,
    result_callback: C,
) where
    C: FnOnce(StatusCode) + 'static,
    R: DeserializeOwned,
    MV: MacVerify,
{
    let logging = request.logging();

    let pending_fetch = match request.start() {
        Ok(future) => future,
        Err(error) => {
            if logging {
                log::debug!("Request failed at init, error: {}", error);
            }
            result_callback(StatusCode::BadRequest);
            transfer_state.lock_mut().stop(StatusCode::FetchFailed);
            return;
        }
    };
    if request.is_load() {
        transfer_state.lock_mut().start_load();
    } else {
        transfer_state.lock_mut().start_store();
    }

    let context = EntityFetchContext {
        logging,
        messages,
        storage_entity,
    };

    spawn_local(async move {
        let status = execute_entity_fetch::<_, MV>(pending_fetch, context).await;
        result_callback(status);
        transfer_state.lock_mut().stop(status);
    });
}

async fn execute_entity_fetch<E, MV>(
    pending_fetch: PendingFetch,
    EntityFetchContext {
        logging,
        messages,
        storage_entity,
    }: EntityFetchContext<E>,
) -> StatusCode
where
    E: Entity + DeserializeOwned,
    MV: MacVerify,
{
    let mut result = execute_fetch::<EntityResponse<E>, MV>(pending_fetch).await;
    match (result.status(), result.take_response()) {
        (status @ StatusCode::FetchTimeout, _) => {
            if logging {
                // TODO: should this warning go also to Messages???
                log::debug!(
                    "Timeout accessing {}.",
                    result.hint().unwrap_or("?unknown url")
                );
            }
            status
        }
        (status @ StatusCode::FetchFailed, _) => {
            if logging {
                // TODO: should this warning go also to Messages???
                log::debug!(
                    "Request failed in execution, error: {}",
                    result.hint().unwrap_or("?unknown")
                );
            }
            status
        }
        (status @ StatusCode::DecodeFailed, _) => {
            if logging {
                // TODO: should this warning go also to Messages???
                log::warn!(
                    "Response decoding failed, error: {}",
                    result.hint().unwrap_or("?unknown")
                );
            }
            status
        }
        (status, None) => status,
        (status, Some(response)) => {
            let (response_messages, received_entity) = response.take();
            messages.set(Messages::with_transport(response_messages));
            if let (Some(entity), Some(response_entity)) = (received_entity, storage_entity) {
                if logging {
                    log::trace!("Request successfully loaded entity");
                }
                response_entity.set(Some(entity));
            }
            status
        }
    }
}

impl<E, MV> Default for EntityStore<E, MV>
where
    E: Entity + Serialize + DeserializeOwned + 'static,
    MV: MacVerify,
{
    fn default() -> Self {
        Self::new_empty()
    }
}

impl<E, MV> From<&EntityStore<E, MV>> for MutableOption<E>
where
    E: Entity + Serialize + DeserializeOwned + 'static,
    MV: MacVerify,
{
    fn from(store: &EntityStore<E, MV>) -> Self {
        store.entity().clone()
    }
}

impl<E, MV> From<&EntityStore<E, MV>> for Mutable<Messages>
where
    E: Entity + Serialize + DeserializeOwned + 'static,
    MV: MacVerify,
{
    fn from(store: &EntityStore<E, MV>) -> Self {
        store.messages().clone()
    }
}

struct EntityFetchContext<E> {
    logging: bool,
    messages: Mutable<Messages>,
    storage_entity: Option<MutableOption<E>>,
}
