use serde::{Deserialize, Serialize};

use super::FIELD_COUNT;

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct TableFieldsState {
    pub(super) visible: [bool; FIELD_COUNT],
}

impl Default for TableFieldsState {
    fn default() -> Self {
        Self {
            visible: [true; FIELD_COUNT],
        }
    }
}
