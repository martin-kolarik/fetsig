use futures_signals::{
    map_ref,
    signal::{Signal, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CollectionState {
    #[default]
    Empty,
    NotEmpty,
    Pending,
}

impl CollectionState {
    pub fn empty(&self) -> bool {
        matches!(*self, Self::Empty)
    }

    pub fn empty_pending(&self) -> bool {
        matches!(*self, Self::Empty | Self::Pending)
    }

    pub fn not_empty(&self) -> bool {
        matches!(*self, Self::NotEmpty)
    }

    pub fn not_empty_pending(&self) -> bool {
        matches!(*self, Self::NotEmpty | Self::Pending)
    }

    pub fn pending(&self) -> bool {
        matches!(*self, Self::Pending)
    }
}

pub fn combine_collection_states_2<S1, S2>(cs1: S1, cs2: S2) -> impl Signal<Item = CollectionState>
where
    S1: Signal<Item = CollectionState>,
    S2: Signal<Item = CollectionState>,
{
    map_ref!(
        cs1, cs2 => {
            match (cs1, cs2) {
                (CollectionState::Pending, _) | (_, CollectionState::Pending) => CollectionState::Pending,
                (CollectionState::NotEmpty, _) | (_, CollectionState::NotEmpty) => CollectionState::NotEmpty,
                (CollectionState::Empty, CollectionState::Empty) => CollectionState::Empty,
            }
        }
    )
}

pub fn combine_collection_states_3<S1, S2, S3>(
    cs1: S1,
    cs2: S2,
    cs3: S3,
) -> impl Signal<Item = CollectionState>
where
    S1: Signal<Item = CollectionState>,
    S2: Signal<Item = CollectionState>,
    S3: Signal<Item = CollectionState>,
{
    map_ref!(
        cs1, cs2, cs3 => {
            match (cs1, cs2, cs3) {
                (CollectionState::Pending, _, _)
                | (_, CollectionState::Pending, _)
                | (_, _, CollectionState::Pending) => CollectionState::Pending,
                (CollectionState::NotEmpty, _, _)
                | (_, CollectionState::NotEmpty, _)
                | (_, _, CollectionState::NotEmpty) => CollectionState::NotEmpty,
                (CollectionState::Empty, CollectionState::Empty, CollectionState::Empty) => CollectionState::Empty,
            }
        }
    )
}

pub fn collection_state_from_vec<T, S>(
    vec: &MutableVec<T>,
    pending: S,
) -> impl Signal<Item = CollectionState> + use<T, S>
where
    T: Clone,
    S: Signal<Item = bool>,
{
    let empty = vec.signal_vec_cloned().is_empty();
    map_ref! {
        pending, empty => (*pending, *empty)
    }
    .map(|state| match state {
        (true, _) => CollectionState::Pending,
        (false, true) => CollectionState::Empty,
        (false, false) => CollectionState::NotEmpty,
    })
}
