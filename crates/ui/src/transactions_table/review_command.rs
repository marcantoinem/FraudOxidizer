use model::data::human_review_status::HumanReviewStatus;
use model::data::transaction::Transaction;
use model::process::card_statistics::FraudFactor;

#[derive(Clone)]
pub(super) struct ReviewUpdate {
    pub(super) transaction_index: usize,
    pub(super) before_status: HumanReviewStatus,
    pub(super) after_status: HumanReviewStatus,
    pub(super) before_factors: Vec<FraudFactor>,
    pub(super) after_factors: Vec<FraudFactor>,
}

#[derive(Clone)]
pub(super) enum ReviewCommand {
    SetHumanReviewStatus { update: ReviewUpdate },
    BatchSetHumanReviewStatus { updates: Vec<ReviewUpdate> },
}

impl ReviewCommand {
    pub(super) fn apply(&self, rows: &mut [Transaction]) {
        match self {
            Self::SetHumanReviewStatus { update } => {
                if let Some(row) = rows.get_mut(update.transaction_index) {
                    row.human_review_status = update.after_status;
                    row.fraud_factors = update.after_factors.clone();
                }
            }
            Self::BatchSetHumanReviewStatus { updates } => {
                for update in updates {
                    if let Some(row) = rows.get_mut(update.transaction_index) {
                        row.human_review_status = update.after_status;
                        row.fraud_factors = update.after_factors.clone();
                    }
                }
            }
        }

        // Review decisions can strengthen/weaken device/IP propagation confidence.
        model::process::passes::fraudulent_identity_link::apply_to_items(rows);
    }

    pub(super) fn undo(&self, rows: &mut [Transaction]) {
        match self {
            Self::SetHumanReviewStatus { update } => {
                if let Some(row) = rows.get_mut(update.transaction_index) {
                    row.human_review_status = update.before_status;
                    row.fraud_factors = update.before_factors.clone();
                }
            }
            Self::BatchSetHumanReviewStatus { updates } => {
                for update in updates {
                    if let Some(row) = rows.get_mut(update.transaction_index) {
                        row.human_review_status = update.before_status;
                        row.fraud_factors = update.before_factors.clone();
                    }
                }
            }
        }

        // Keep propagated identity-link factors consistent after undo.
        model::process::passes::fraudulent_identity_link::apply_to_items(rows);
    }
}
