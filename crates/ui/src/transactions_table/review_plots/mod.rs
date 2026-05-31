mod burst_histogram;
mod burst_timeline;
mod card_amount_deviation_chart;
mod category_price_deviation_chart;
mod foreign_trip_table;
mod merchant_ring_chart;

pub(super) use burst_histogram::burst_histogram_slot;
pub(super) use burst_timeline::burst_timeline_slot;
pub(super) use card_amount_deviation_chart::card_amount_deviation_slot;
pub(super) use category_price_deviation_chart::category_price_deviation_slot;
pub(super) use foreign_trip_table::foreign_trip_table_slot;
pub(super) use merchant_ring_chart::merchant_ring_slot;
