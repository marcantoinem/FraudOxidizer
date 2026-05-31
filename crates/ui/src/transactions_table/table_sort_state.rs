use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Default)]
pub(super) struct TableSortState {
    pub(super) field_idx: Option<usize>,
    pub(super) descending: bool,
}
