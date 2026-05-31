use model::data::human_review_status::HumanReviewStatus;
use model::data::transaction::Transaction;

#[derive(Clone)]
pub(super) enum ReviewCommand {
    SetHumanReviewStatus {
        transaction_index: usize,
        before: HumanReviewStatus,
        after: HumanReviewStatus,
    },
    BatchSetHumanReviewStatus {
        updates: Vec<(usize, HumanReviewStatus, HumanReviewStatus)>,
    },
}

impl ReviewCommand {
    pub(super) fn apply(&self, rows: &mut [Transaction]) {
        match self {
            Self::SetHumanReviewStatus {
                transaction_index,
                after,
                ..
            } => {
                if let Some(row) = rows.get_mut(*transaction_index) {
                    row.human_review_status = *after;
                }
            }
            Self::BatchSetHumanReviewStatus { updates } => {
                for (transaction_index, _, after) in updates {
                    if let Some(row) = rows.get_mut(*transaction_index) {
                        row.human_review_status = *after;
                    }
                }
            }
        }
    }

    pub(super) fn undo(&self, rows: &mut [Transaction]) {
        match self {
            Self::SetHumanReviewStatus {
                transaction_index,
                before,
                ..
            } => {
                if let Some(row) = rows.get_mut(*transaction_index) {
                    row.human_review_status = *before;
                }
            }
            Self::BatchSetHumanReviewStatus { updates } => {
                for (transaction_index, before, _) in updates {
                    if let Some(row) = rows.get_mut(*transaction_index) {
                        row.human_review_status = *before;
                    }
                }
            }
        }
    }

    pub(super) fn transaction_index(&self) -> usize {
        match self {
            Self::SetHumanReviewStatus {
                transaction_index, ..
            } => *transaction_index,
            Self::BatchSetHumanReviewStatus { updates } => {
                updates.first().map(|(idx, _, _)| *idx).unwrap_or(0)
            }
        }
    }
}
