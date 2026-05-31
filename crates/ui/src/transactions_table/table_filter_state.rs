use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub(super) struct TableFilterState {
    pub(super) card_id_query: String,
    pub(super) autocomplete_open: bool,
    pub(super) reviewed_or_marked_only: bool,
}

impl Default for TableFilterState {
    fn default() -> Self {
        Self {
            card_id_query: String::new(),
            autocomplete_open: false,
            reviewed_or_marked_only: true,
        }
    }
}
