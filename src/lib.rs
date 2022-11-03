//! Contains help text for app, config, etc.
//! as well as other modules.

mod command;
mod scraper_utils;
mod set_data;
mod query;

#[doc(hidden)]
pub use command::Leget;

const MIN_YEAR_BRICK_ECONOMY: u16 = 1949;
