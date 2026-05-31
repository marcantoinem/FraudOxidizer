#[derive(Clone, Copy)]
pub(super) enum CardIdFilter {
    Any,
    Exact(u64),
    Invalid,
}

pub(super) fn parse_card_id_filter(query: &str) -> CardIdFilter {
    let normalized = normalize_card_id_query(query);
    if normalized.is_empty() {
        return CardIdFilter::Any;
    }

    match normalized.parse::<u64>() {
        Ok(card_id) => CardIdFilter::Exact(card_id),
        Err(_) => CardIdFilter::Invalid,
    }
}

pub(super) fn normalize_card_id_query(query: &str) -> String {
    let normalized = query.trim().to_ascii_lowercase();
    normalized
        .strip_prefix("card_")
        .unwrap_or(&normalized)
        .to_owned()
}

pub(super) fn card_id_matches_filter(card_id: u64, filter: CardIdFilter) -> bool {
    match filter {
        CardIdFilter::Any => true,
        CardIdFilter::Exact(selected) => card_id == selected,
        CardIdFilter::Invalid => false,
    }
}
