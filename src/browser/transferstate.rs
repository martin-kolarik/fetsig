use futures_signals::signal::{Mutable, Signal, SignalExt};

use crate::StatusCode;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    #[default]
    Empty,
    PendingLoad,
    PendingStore,
    Loaded(StatusCode),
    Stored(StatusCode),
}

impl TransferState {
    pub fn pending(&self) -> bool {
        matches!(*self, Self::PendingLoad | Self::PendingStore)
    }

    pub fn as_load(self) -> OperationState {
        match self {
            Self::Empty | Self::PendingStore | Self::Stored(_) => OperationState::Empty,
            Self::PendingLoad => OperationState::Pending,
            Self::Loaded(status) => OperationState::Completed(status),
        }
    }

    #[inline]
    pub fn loaded(&self) -> bool {
        matches!(*self, Self::Loaded(status) if status.is_success())
    }

    pub fn loaded_status(&self) -> Option<StatusCode> {
        if let Self::Loaded(status) = self {
            Some(*status)
        } else {
            None
        }
    }

    pub fn as_store(self) -> OperationState {
        match self {
            Self::Empty | Self::PendingLoad | Self::Loaded(_) => OperationState::Empty,
            Self::PendingStore => OperationState::Pending,
            Self::Stored(status) => OperationState::Completed(status),
        }
    }

    pub fn stored(&self) -> bool {
        matches!(*self, Self::Stored(status) if status.is_success())
    }

    pub fn stored_status(&self) -> Option<StatusCode> {
        if let Self::Stored(status) = self {
            Some(*status)
        } else {
            None
        }
    }

    pub fn not_completed(&self) -> bool {
        !matches!(*self, Self::Loaded(..) | Self::Stored(..))
    }

    pub fn not_error(&self) -> bool {
        !matches!(*self, Self::Loaded(status) | Self::Stored(status) if status.is_failure())
    }

    pub fn reset_error(&mut self) {
        *self = match self {
            Self::Loaded(..) => Self::Loaded(StatusCode::Ok),
            Self::Stored(..) => Self::Stored(StatusCode::Ok),
            _ => *self,
        }
    }

    pub(crate) fn start_load(&mut self) {
        *self = Self::PendingLoad;
    }

    pub(crate) fn start_store(&mut self) {
        *self = Self::PendingStore;
    }

    pub(crate) fn stop(&mut self, status: StatusCode) {
        *self = match *self {
            Self::PendingLoad | Self::Loaded(..) => Self::Loaded(status),
            Self::PendingStore | Self::Stored(..) => Self::Stored(status),
            _ => Self::Loaded(StatusCode::FetchFailed),
        };
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    #[default]
    Empty,
    Pending,
    Completed(StatusCode),
}

impl OperationState {
    pub fn pending(&self) -> bool {
        matches!(*self, Self::Pending)
    }

    pub fn completed(&self) -> bool {
        matches!(*self, Self::Completed(status) if status.is_success())
    }

    pub fn error(&self) -> bool {
        matches!(*self, Self::Completed(status) if status.is_failure())
    }

    pub fn status(&self) -> Option<StatusCode> {
        if let Self::Completed(status) = self {
            Some(*status)
        } else {
            None
        }
    }

    #[inline]
    pub fn not_completed(&self) -> bool {
        !self.completed()
    }

    #[inline]
    pub fn not_error(&self) -> bool {
        !self.error()
    }
}

#[inline]
pub fn loaded(state: &Mutable<TransferState>) -> bool {
    state.get().loaded()
}

#[inline]
pub fn loaded_signal(state: &Mutable<TransferState>) -> impl Signal<Item = bool> + use<> {
    state.signal_ref(TransferState::loaded).dedupe()
}

#[inline]
pub fn loaded_status(state: &Mutable<TransferState>) -> Option<StatusCode> {
    state.get().loaded_status()
}

#[inline]
pub fn loaded_status_signal(
    state: &Mutable<TransferState>,
) -> impl Signal<Item = Option<StatusCode>> + use<> {
    state.signal_ref(TransferState::loaded_status).dedupe()
}

#[inline]
pub fn loaded_state(state: &Mutable<TransferState>) -> OperationState {
    state.get().as_load()
}

#[inline]
pub fn loaded_state_signal(
    state: &Mutable<TransferState>,
) -> impl Signal<Item = OperationState> + use<> {
    state.signal().map(TransferState::as_load).dedupe()
}

#[inline]
pub fn stored(state: &Mutable<TransferState>) -> bool {
    state.get().stored()
}

#[inline]
pub fn stored_signal(state: &Mutable<TransferState>) -> impl Signal<Item = bool> + use<> {
    state.signal_ref(TransferState::stored).dedupe()
}

#[inline]
pub fn stored_status(state: &Mutable<TransferState>) -> Option<StatusCode> {
    state.get().stored_status()
}

#[inline]
pub fn stored_status_signal(
    state: &Mutable<TransferState>,
) -> impl Signal<Item = Option<StatusCode>> + use<> {
    state.signal_ref(TransferState::stored_status).dedupe()
}

#[inline]
pub fn stored_state(state: &Mutable<TransferState>) -> OperationState {
    state.get().as_store()
}

#[inline]
pub fn stored_state_signal(
    state: &Mutable<TransferState>,
) -> impl Signal<Item = OperationState> + use<> {
    state.signal().map(TransferState::as_store).dedupe()
}

#[inline]
pub fn pending(state: &Mutable<TransferState>) -> bool {
    state.get().pending()
}

#[inline]
pub fn pending_signal(state: &Mutable<TransferState>) -> impl Signal<Item = bool> + use<> {
    state.signal_ref(TransferState::pending).dedupe()
}
