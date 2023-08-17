use std::marker::PhantomData;

use futures_signals::{
    map_ref,
    signal::{Mutable, Signal},
    signal_vec::{MutableSignalVec, MutableVec, MutableVecLockMut, MutableVecLockRef, SignalVec},
};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen_futures::spawn_local;

use crate::{MacSign, MacVerify, MediaType, NoMac, StatusCode, HEADER_SIGNATURE};

use super::{
    common::{execute_fetch, PendingFetch},
    request::Request,
    transferstate::TransferState,
    CollectionState,
};

#[derive(Debug)]
pub struct CollectionStore<E, MV = NoMac> {
    transfer_state: Mutable<TransferState>,
    messages: Mutable<Messages>,
    paging: Mutable<Paging>,
    collection: MutableVec<E>,
    pmv: PhantomData<MV>,
}

impl<E, MV> CollectionStore<E, MV>
where
    E: Clone + Serialize + DeserializeOwned,
    MV: MacVerify,
{
    pub fn new_empty() -> Self {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
            messages: Mutable::new(Messages::default()),
            paging: Mutable::new(Paging::default()),
            collection: MutableVec::new_with_values(vec![]),
            pmv: PhantomData,
        }
    }

    pub fn new_value(collection: Vec<E>) -> Self {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
            messages: Mutable::new(Messages::default()),
            paging: Mutable::new(Paging::default()),
            collection: MutableVec::new_with_values(collection),
            pmv: PhantomData,
        }
    }

    pub fn reset_to_empty(&self) {
        self.init(TransferState::Empty);
    }

    pub fn invalidate(&self) {
        self.transfer_state.set_neq(TransferState::Empty);
    }

    pub fn transfer_state(&self) -> TransferState {
        self.transfer_state.get()
    }

    pub fn set_transfer_state(&self, transfer_state: TransferState) {
        self.transfer_state.set_neq(transfer_state);
    }

    pub fn reset_transfer_error(&self) {
        self.transfer_state.lock_mut().reset_error();
    }

    fn init(&self, transfer_state: TransferState) {
        self.transfer_state.set_neq(transfer_state);
        self.messages.set(Messages::default());
        self.paging.set(Paging::default());
        self.reset();
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

    pub fn empty_signal(&self) -> impl Signal<Item = bool> {
        self.collection.signal_vec_cloned().is_empty().dedupe()
    }

    pub fn collection_state_signal(&self) -> impl Signal<Item = CollectionState> {
        collection_state_signal(self.pending_signal(), self.empty_signal())
    }

    pub fn messages(&self) -> &Mutable<Messages> {
        &self.messages
    }

    pub fn paging(&self) -> &Mutable<Paging> {
        &self.paging
    }

    pub fn collection(&self) -> &MutableVec<E> {
        &self.collection
    }

    pub fn is_empty(&self) -> bool {
        self.collection.lock_ref().is_empty()
    }

    pub fn any<F>(&self, f: F) -> bool
    where
        F: Fn(&E) -> bool,
    {
        self.collection.lock_ref().iter().any(f)
    }

    pub fn all<F>(&self, f: F) -> bool
    where
        F: Fn(&E) -> bool,
    {
        self.collection.lock_ref().iter().all(f)
    }

    pub fn find<F>(&self, f: F) -> Option<E>
    where
        F: Fn(&E) -> bool,
    {
        self.find_map(|e| f(e).then(|| e.clone()))
    }

    pub fn find_map<F, U>(&self, f: F) -> Option<U>
    where
        F: Fn(&E) -> Option<U>,
    {
        self.collection.lock_ref().iter().find_map(f)
    }

    pub fn lock_ref(&self) -> MutableVecLockRef<E> {
        self.collection.lock_ref()
    }

    pub fn lock_mut(&self) -> MutableVecLockMut<E> {
        self.collection.lock_mut()
    }

    pub fn inspect<F>(&self, f: F)
    where
        F: FnOnce(MutableVecLockRef<E>),
    {
        f(self.collection.lock_ref())
    }

    pub fn inspect_mut<F>(&self, f: F)
    where
        F: FnOnce(MutableVecLockMut<E>),
    {
        f(self.collection.lock_mut())
    }

    pub fn find_inspect_mut<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        P: FnMut(&E) -> bool,
        F: FnMut(&mut E) -> bool,
    {
        self.collection.find_inspect_mut(predicate, f)
    }

    pub fn map<F, U>(&self, f: F) -> U
    where
        F: FnOnce(MutableVecLockRef<E>) -> U,
    {
        f(self.collection.lock_ref())
    }

    pub fn map_mut<F, U>(&self, f: F) -> U
    where
        F: FnOnce(MutableVecLockMut<E>) -> U,
    {
        f(self.collection.lock_mut())
    }

    pub fn get(&self) -> Vec<E> {
        self.collection.lock_ref().to_vec()
    }

    pub fn set<P>(&self, predicate: P, item: E)
    where
        P: FnMut(&E) -> bool,
    {
        self.collection.inspect_mut(|collection| {
            if let Some(index) = collection.iter().position(predicate) {
                collection.set_cloned(index, item);
            }
        });
    }

    pub fn set_or_add<P>(&self, predicate: P, item: E)
    where
        P: FnMut(&E) -> bool,
    {
        self.collection
            .inspect_mut(|collection| match collection.iter().position(predicate) {
                Some(index) => collection.set_cloned(index, item),
                None => collection.push_cloned(item),
            });
    }

    pub fn set_or_insert<K, O>(&self, mut sort_key: K, item: E)
    where
        K: FnMut(&E) -> O,
        O: Ord,
    {
        self.collection.inspect_mut(|collection| {
            match collection.binary_search_by_key(&sort_key(&item), sort_key) {
                Ok(index) => collection.set_cloned(index, item),
                Err(index) => collection.insert_cloned(index, item),
            }
        });
    }

    pub fn remove<P>(&self, predicate: P)
    where
        P: FnMut(&E) -> bool,
    {
        self.collection.inspect_mut(|collection| {
            if let Some(index) = collection.iter().position(predicate) {
                collection.remove(index);
            }
        });
    }

    pub fn replace(&self, values: Vec<E>) -> Vec<E> {
        let mut collection = self.collection.lock_mut();
        let current = collection.drain(..).collect();
        collection.replace_cloned(values);
        current
    }

    pub fn signal_map<F, U>(&self, f: F) -> impl Signal<Item = U>
    where
        F: FnMut(&[E]) -> U,
    {
        self.collection.signal_vec_cloned().to_signal_map(f)
    }

    pub fn signal_vec(&self) -> MutableSignalVec<E> {
        self.collection.signal_vec_cloned()
    }

    pub fn signal_vec_filter<F>(&self, f: F) -> impl SignalVec<Item = E>
    where
        F: FnMut(&E) -> bool,
    {
        self.collection.signal_vec_filter(f)
    }

    pub fn signal_vec_filter_signal<F, U>(&self, f: F) -> impl SignalVec<Item = E>
    where
        F: FnMut(&E) -> U,
        U: Signal<Item = bool>,
    {
        self.collection.signal_vec_filter_signal(f)
    }

    pub fn signal_vec_map<F, U>(&self, f: F) -> impl SignalVec<Item = U>
    where
        F: FnMut(E) -> U,
    {
        self.collection.signal_vec_cloned().map(f)
    }

    pub fn signal_vec_map_signal<F, U>(&self, f: F) -> impl SignalVec<Item = U::Item>
    where
        F: FnMut(E) -> U,
        U: Signal,
    {
        self.collection.signal_vec_cloned().map_signal(f)
    }

    pub fn signal_vec_filter_map<F, U>(&self, f: F) -> impl SignalVec<Item = U>
    where
        F: FnMut(E) -> Option<U>,
    {
        self.collection.signal_vec_cloned().filter_map(f)
    }

    pub fn reset(&self) {
        self.collection.lock_mut().clear()
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
            self.paging.clone(),
            self.collection.clone(),
            result_callback,
        );
    }

    pub fn store<MS, C>(&self, request: Request<'_>, result_callback: C)
    where
        MS: MacSign,
        C: FnOnce(StatusCode) + 'static,
    {
        let mut request = request.with_is_load(false);
        if request.logging() {
            log::debug!("Request to update {}", request.url());

            if request.method().is_load() {
                log::warn!(
                    "Store request unexpectedly uses load verb {:?}",
                    request.method().as_str()
                );
            }
        }

        {
            // scope around vector and collection borrow
            let collection = self.lock_ref();
            if !collection.is_empty() {
                let media_type = match request.media_type() {
                    Some(media_type @ MediaType::Json | media_type @ MediaType::Postcard) => {
                        media_type
                    }
                    _ => {
                        if request.logging() {
                            log::warn!("Request failed as unsupported media type is requested");
                        }
                        *self.messages.lock_mut() = Messages::from_service_error(
                            "Request failed as unsupported media type is requested",
                        );
                        self.transfer_state
                            .lock_mut()
                            .stop(StatusCode::UnsupportedMediaType);
                        return;
                    }
                };

                let content = collection.to_vec();
                let bytes = match media_type {
                    MediaType::Json => content.to_json(),
                    MediaType::Postcard => content.to_postcard(),
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
        }

        fetch::<_, _, MV>(
            request,
            self.transfer_state.clone(),
            self.messages.clone(),
            self.paging.clone(),
            self.collection.clone(),
            result_callback,
        );
    }
}

fn fetch<E, C, MV>(
    request: Request<'_>,
    transfer_state: Mutable<TransferState>,
    messages: Mutable<Messages>,
    paging: Mutable<Paging>,
    collection: MutableVec<E>,
    result_callback: C,
) where
    E: Clone + DeserializeOwned,
    C: FnOnce(StatusCode) + 'static,
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

    let context = CollectionFetchContext {
        logging,
        messages,
        paging,
        collection,
    };

    spawn_local(async move {
        let status = execute_collection_fetch::<_, MV>(pending_fetch, context).await;
        result_callback(status);
        transfer_state.lock_mut().stop(status);
    });
}

async fn execute_collection_fetch<E, MV>(
    pending_fetch: PendingFetch,
    CollectionFetchContext {
        logging,
        messages,
        paging,
        collection,
    }: CollectionFetchContext<E>,
) -> StatusCode
where
    E: Entity + DeserializeOwned + 'static,
    MV: MacVerify,
{
    let mut result = execute_fetch::<CollectionResponse<E>, MV>(pending_fetch).await;
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
            let (response_messages, response_paging, response_entities) = response.take();
            messages.set(Messages::with_transport(response_messages));
            if let Some(entities) = response_entities {
                if logging {
                    log::trace!("Request successfully fetched collection");
                }
                collection.lock_mut().replace_cloned(entities);
            }
            *paging.lock_mut() = response_paging;
            status
        }
    }
}

impl<E, MV> Default for CollectionStore<E, MV>
where
    E: Entity + Serialize + DeserializeOwned + 'static,
    MV: MacVerify,
{
    fn default() -> Self {
        Self::new_empty()
    }
}

struct CollectionFetchContext<E> {
    logging: bool,
    messages: Mutable<Messages>,
    paging: Mutable<Paging>,
    collection: MutableVec<E>,
}

pub fn collection_state_signal<P, E>(pending: P, empty: E) -> impl Signal<Item = CollectionState>
where
    P: Signal<Item = bool>,
    E: Signal<Item = bool>,
{
    map_ref! {
        pending, empty => {
            match (pending, empty) {
                (true, _) => CollectionState::Pending,
                (false, true) => CollectionState::Empty,
                (false, false) => CollectionState::NotEmpty,
            }
        }
    }
    .dedupe()
}
