use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals_ext::{MutableExt, MutableOption};
use log::debug;
use serde::de::DeserializeOwned;
use smol_str::SmolStr;

use crate::{Messages, NoMac, StatusCode};

use super::{fetch, request::Request, transferstate::TransferState};

#[derive(Default)]
pub struct UploadStore {
    transfer_state: Mutable<TransferState>,
}

impl UploadStore {
    pub fn new() -> Self {
        Self {
            transfer_state: Mutable::new(TransferState::Empty),
        }
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

    pub fn stored(&self) -> bool {
        self.transfer_state.map(TransferState::stored)
    }

    pub fn stored_signal(&self) -> impl Signal<Item = bool> + use<> {
        self.transfer_state.signal_ref(|state| state.stored())
    }

    pub fn stored_status(&self) -> Option<StatusCode> {
        self.transfer_state.map(TransferState::stored_status)
    }

    pub fn stored_status_signal(&self) -> impl Signal<Item = Option<StatusCode>> + use<> {
        self.transfer_state
            .signal_ref(TransferState::stored_status)
            .dedupe()
    }

    pub fn pending(&self) -> bool {
        self.transfer_state.map(TransferState::pending)
    }

    pub fn pending_signal(&self) -> impl Signal<Item = bool> + use<> {
        self.transfer_state.signal_ref(|state| state.pending())
    }

    pub fn store<C>(&self, request: Request<'_>, response_messages: Messages, result_callback: C)
    where
        C: FnOnce(StatusCode) + 'static,
    {
        self.do_store::<SmolStr, _>(request, None, response_messages, result_callback)
    }

    pub fn store_with_response<R, C>(
        &self,
        request: Request<'_>,
        response_entity: MutableOption<R>,
        response_messages: Messages,
        result_callback: C,
    ) where
        R: DeserializeOwned + 'static,
        C: FnOnce(StatusCode) + 'static,
    {
        self.do_store::<_, _>(
            request,
            Some(response_entity),
            response_messages,
            result_callback,
        );
    }

    fn do_store<R, C>(
        &self,
        request: Request<'_>,
        response_entity: Option<MutableOption<R>>,
        response_messages: Messages,
        result_callback: C,
    ) where
        C: FnOnce(StatusCode) + 'static,
        R: DeserializeOwned + 'static,
    {
        if request.logging() {
            debug!("Request to store {}", request.url());
        }
        fetch::<_, _, NoMac>(
            request,
            self.transfer_state.clone(),
            response_messages,
            response_entity,
            result_callback,
        );
    }
}
