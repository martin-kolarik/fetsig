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
