#[derive(Debug)]
pub enum HumanReviewStatus {
    NotNeeded,
    NeedCheck,
    FalsePositive,
    TruePositive,
}

impl HumanReviewStatus {
    pub fn likely_fraud_override(&self) -> Option<bool> {
        match self {
            Self::FalsePositive => Some(false),
            Self::TruePositive => Some(true),
            Self::NotNeeded | Self::NeedCheck => None,
        }
    }
}
