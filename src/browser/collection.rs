use std::{cmp, marker::PhantomData};

use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{
        MutableSignalVec, MutableVec, MutableVecLockMut, MutableVecLockRef, SignalVec, SignalVecExt,
    },
};
use futures_signals_ext::{MutableExt, MutableVecExt};
use log::{debug, error, trace, warn};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen_futures::spawn_local;

#[cfg(feature = "json")]
use crate::JSONSerialize;
#[cfg(any(feature = "json", feature = "postcard"))]
use crate::MediaType;
#[cfg(feature = "postcard")]
use crate::PostcardSerialize;
use crate::{
    CollectionResponse, MacSign, MacVerify, Messages, NoMac, Paging, StatusCode, HEADER_SIGNATURE,
};

use super::{
    common::{execute_fetch, PendingFetch},
    request::Request,
    transferstate::TransferState,
    CollectionState,
};

#[derive(Debug)]
pub struct CollectionStore<E, MV = NoMac> {
    transfer_state: Mutable<TransferState>,
    messages: Messages,
    paging: Mutable<Paging>,
    collection: MutableVec<E>,
    pmv: PhantomData<MV>,
}

impl<E, MV> CollectionStore<E, MV> {
    pub fn new_empty() -> Self {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
            messages: Messages::new(),
            paging: Mutable::new(Paging::default()),
            collection: MutableVec::new_with_values(vec![]),
            pmv: PhantomData,
        }
    }

    pub fn new_value(collection: Vec<E>) -> Self {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
            messages: Messages::new(),
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
        self.messages.clear_all();
        self.paging.set(Paging::default());
        self.reset();
    }

    pub fn reset(&self) {
        self.collection.lock_mut().clear()
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

    pub fn collection(&self) -> &MutableVec<E> {
        &self.collection
    }

    pub fn messages(&self) -> &Messages {
        &self.messages
    }

    pub fn paging(&self) -> &Mutable<Paging> {
        &self.paging
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

    pub fn lock_ref(&self) -> MutableVecLockRef<E> {
        self.collection.lock_ref()
    }

    pub fn lock_mut(&self) -> MutableVecLockMut<E> {
        self.collection.lock_mut()
    }

    pub fn inspect<F>(&self, f: F)
    where
        F: FnOnce(&[E]),
    {
        self.collection.inspect(f)
    }

    pub fn inspect_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut MutableVecLockMut<E>),
    {
        self.collection.inspect_mut(f)
    }

    pub fn find_map<F, U>(&self, f: F) -> Option<U>
    where
        F: Fn(&E) -> Option<U>,
    {
        self.collection.lock_ref().iter().find_map(f)
    }

    pub fn map_vec<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&[E]) -> U,
    {
        self.collection.map_vec(f)
    }

    pub fn map_vec_mut<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut MutableVecLockMut<E>) -> U,
    {
        self.collection.map_vec_mut(f)
    }

    pub fn remove<P>(&self, predicate: P) -> bool
    where
        P: FnMut(&E) -> bool,
    {
        self.collection.find_remove(predicate)
    }
}

impl<E, MV> CollectionStore<E, MV>
where
    E: Copy,
{
    pub fn empty_signal(&self) -> impl Signal<Item = bool> {
        self.collection.signal_vec().is_empty().dedupe()
    }

    pub fn collection_state_signal(&self) -> impl Signal<Item = CollectionState> {
        collection_state_signal(self.pending_signal(), self.empty_signal())
    }

    pub fn find<F>(&self, f: F) -> Option<E>
    where
        F: Fn(&E) -> bool,
    {
        self.find_map(|e| f(e).then_some(*e))
    }

    pub fn find_inspect_mut<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        P: FnMut(&E) -> bool,
        F: FnMut(&mut E) -> bool,
    {
        self.collection.find_inspect_mut(predicate, f)
    }

    pub fn find_set<P>(&self, predicate: P, item: E) -> bool
    where
        P: FnMut(&E) -> bool,
    {
        self.collection.find_set(predicate, item)
    }

    pub fn find_set_or_add<P>(&self, predicate: P, item: E)
    where
        P: FnMut(&E) -> bool,
    {
        self.collection.find_set_or_add(predicate, item);
    }

    pub fn find_set_or_insert<F>(&self, cmp: F, item: E)
    where
        F: FnMut(&E) -> cmp::Ordering,
    {
        self.collection.find_set_or_insert(cmp, item);
    }

    pub fn replace(&self, values: Vec<E>) -> Vec<E> {
        let mut collection = self.collection.lock_mut();
        let current = collection.drain(..).collect();
        collection.replace(values);
        current
    }

    pub fn signal_map<F, U>(&self, f: F) -> impl Signal<Item = U>
    where
        F: FnMut(&[E]) -> U,
    {
        self.collection.signal_vec().to_signal_map(f)
    }

    pub fn signal_vec(&self) -> MutableSignalVec<E> {
        self.collection.signal_vec()
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
        self.collection.signal_vec().map(f)
    }

    pub fn signal_vec_map_signal<F, U>(&self, f: F) -> impl SignalVec<Item = U::Item>
    where
        F: FnMut(E) -> U,
        U: Signal,
    {
        self.collection.signal_vec().map_signal(f)
    }

    pub fn signal_vec_filter_map<F, U>(&self, f: F) -> impl SignalVec<Item = U>
    where
        F: FnMut(E) -> Option<U>,
    {
        self.collection.signal_vec().filter_map(f)
    }
}

impl<E, MV> CollectionStore<E, MV>
where
    E: Clone,
{
    pub fn empty_signal_cloned(&self) -> impl Signal<Item = bool> {
        self.collection.signal_vec_cloned().is_empty().dedupe()
    }

    pub fn collection_state_signal_cloned(&self) -> impl Signal<Item = CollectionState> {
        collection_state_signal(self.pending_signal(), self.empty_signal_cloned())
    }

    pub fn find_cloned<F>(&self, f: F) -> Option<E>
    where
        F: Fn(&E) -> bool,
    {
        self.find_map(|e| f(e).then(|| e.clone()))
    }

    pub fn find_inspect_mut_cloned<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        P: FnMut(&E) -> bool,
        F: FnMut(&mut E) -> bool,
    {
        self.collection.find_inspect_mut_cloned(predicate, f)
    }

    pub fn get_cloned(&self) -> Vec<E> {
        self.collection.lock_ref().to_vec()
    }

    pub fn find_set_cloned<P>(&self, predicate: P, item: E) -> bool
    where
        P: FnMut(&E) -> bool,
    {
        self.collection.find_set_cloned(predicate, item)
    }

    pub fn find_set_or_add_cloned<P>(&self, predicate: P, item: E)
    where
        P: FnMut(&E) -> bool,
    {
        self.collection.find_set_or_add_cloned(predicate, item);
    }

    pub fn find_set_or_insert_cloned<F>(&self, cmp: F, item: E)
    where
        F: FnMut(&E) -> cmp::Ordering,
    {
        self.collection.find_set_or_insert_cloned(cmp, item);
    }

    pub fn replace_cloned(&self, values: Vec<E>) -> Vec<E> {
        let mut collection = self.collection.lock_mut();
        let current = collection.drain(..).collect();
        collection.replace_cloned(values);
        current
    }

    pub fn signal_map_cloned<F, U>(&self, f: F) -> impl Signal<Item = U>
    where
        F: FnMut(&[E]) -> U,
    {
        self.collection.signal_vec_cloned().to_signal_map(f)
    }

    pub fn signal_vec_cloned(&self) -> MutableSignalVec<E> {
        self.collection.signal_vec_cloned()
    }

    pub fn signal_vec_filter_cloned<F>(&self, f: F) -> impl SignalVec<Item = E>
    where
        F: FnMut(&E) -> bool,
    {
        self.collection.signal_vec_filter_cloned(f)
    }

    pub fn signal_vec_filter_signal_cloned<F, U>(&self, f: F) -> impl SignalVec<Item = E>
    where
        F: FnMut(&E) -> U,
        U: Signal<Item = bool>,
    {
        self.collection.signal_vec_filter_signal_cloned(f)
    }

    pub fn signal_vec_map_cloned<F, U>(&self, f: F) -> impl SignalVec<Item = U>
    where
        F: FnMut(E) -> U,
    {
        self.collection.signal_vec_cloned().map(f)
    }

    pub fn signal_vec_map_signal_cloned<F, U>(&self, f: F) -> impl SignalVec<Item = U::Item>
    where
        F: FnMut(E) -> U,
        U: Signal,
    {
        self.collection.signal_vec_cloned().map_signal(f)
    }

    pub fn signal_vec_filter_map_cloned<F, U>(&self, f: F) -> impl SignalVec<Item = U>
    where
        F: FnMut(E) -> Option<U>,
    {
        self.collection.signal_vec_cloned().filter_map(f)
    }
}

impl<E, MV> CollectionStore<E, MV>
where
    E: Clone,
    MV: MacVerify,
{
    pub fn load<C>(&self, request: Request<'_>, result_callback: C)
    where
        E: DeserializeOwned + 'static,
        C: FnOnce(StatusCode) + 'static,
    {
        if self.transfer_state.map(TransferState::loaded) {
            if request.logging() {
                debug!("Request to load {} skipped, using cache", request.url());

                if !request.method().is_load() {
                    warn!(
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
        E: DeserializeOwned + 'static,
        C: FnOnce(StatusCode) + 'static,
    {
        if request.logging() {
            debug!("Request to load {}", request.url());

            if !request.method().is_load() {
                warn!(
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
        E: Serialize + DeserializeOwned + 'static,
        MS: MacSign,
        C: FnOnce(StatusCode) + 'static,
    {
        let mut request = request.with_is_load(false);
        if request.logging() {
            debug!("Request to update {}", request.url());

            if request.method().is_load() {
                warn!(
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
                    #[cfg(feature = "json")]
                    Some(media_type @ MediaType::Json) => media_type,
                    #[cfg(feature = "postcard")]
                    Some(media_type @ MediaType::Postcard) => media_type,
                    _ => {
                        if request.logging() {
                            warn!("Request failed as unsupported media type is requested");
                        }
                        self.messages.replace(Messages::from_service_error(
                            "Request failed as unsupported media type is requested",
                        ));
                        self.transfer_state
                            .lock_mut()
                            .stop(StatusCode::UnsupportedMediaType);
                        return;
                    }
                };

                let content = collection.to_vec();
                let bytes = match media_type {
                    #[cfg(feature = "json")]
                    MediaType::Json => content.to_json(),
                    #[cfg(feature = "postcard")]
                    MediaType::Postcard => content.to_postcard(),
                    _ => {
                        if request.logging() {
                            error!("Unsupported media type requested, unexpected code flow");
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
    messages: Messages,
    paging: Mutable<Paging>,
    collection: MutableVec<E>,
    result_callback: C,
) where
    E: Clone + DeserializeOwned + 'static,
    C: FnOnce(StatusCode) + 'static,
    MV: MacVerify,
{
    let logging = request.logging();

    let pending_fetch = match request.start() {
        Ok(future) => future,
        Err(error) => {
            if logging {
                debug!("Request failed at init, error: {}", error);
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
    E: Clone + DeserializeOwned,
    MV: MacVerify,
{
    let mut result = execute_fetch::<CollectionResponse<E>, MV>(pending_fetch).await;
    match (result.status(), result.take_response()) {
        (status @ StatusCode::FetchTimeout, _) => {
            if logging {
                // TODO: should this warning go also to Messages???
                debug!(
                    "Timeout accessing {}.",
                    result.hint().unwrap_or("?unknown url")
                );
            }
            status
        }
        (status @ StatusCode::FetchFailed, _) => {
            if logging {
                // TODO: should this warning go also to Messages???
                debug!(
                    "Request failed in execution, error: {}",
                    result.hint().unwrap_or("?unknown")
                );
            }
            status
        }
        (status @ StatusCode::DecodeFailed, _) => {
            if logging {
                // TODO: should this warning go also to Messages???
                warn!(
                    "Response decoding failed, error: {}",
                    result.hint().unwrap_or("?unknown")
                );
            }
            status
        }
        (status, None) => status,
        (status, Some(response)) => {
            let (response_entities, response_messages, response_paging) = response.take();
            messages.replace(response_messages);
            if let Some(entities) = response_entities {
                if logging {
                    trace!("Request successfully fetched collection");
                }
                collection.lock_mut().replace_cloned(entities);
            }
            *paging.lock_mut() = response_paging;
            status
        }
    }
}

impl<E, MV> Default for CollectionStore<E, MV> {
    fn default() -> Self {
        Self::new_empty()
    }
}

struct CollectionFetchContext<E> {
    logging: bool,
    messages: Messages,
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
