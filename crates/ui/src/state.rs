#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, Default)]
pub enum Progression {
    #[default]
    ImportCsv,
    CheckProbableFraud,
    ExportCsvView,
}

impl Progression {
    pub fn title(self) -> &'static str {
        match self {
            Self::ImportCsv => "Import CSV",
            Self::CheckProbableFraud => "Human Review",
            Self::ExportCsvView => "Overview",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::ImportCsv => Self::CheckProbableFraud,
            Self::CheckProbableFraud => Self::ExportCsvView,
            Self::ExportCsvView => Self::ExportCsvView,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::ImportCsv => Self::ImportCsv,
            Self::CheckProbableFraud => Self::ImportCsv,
            Self::ExportCsvView => Self::CheckProbableFraud,
        }
    }
}
