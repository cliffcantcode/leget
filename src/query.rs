use crate::MIN_YEAR_BRICK_ECONOMY;
use chrono::offset::Utc;
use chrono::Datelike;

// until method for other impl methods
fn current_year() -> u16 {
    let date = Utc::today();
    date.year().try_into().expect("A u16 of the current year.")
}

// the filters that will be applied to our data
pub struct Query {
    pub years: Option<Vec<u16>>,
    pub set_range: Option<Vec<u32>>,
}

impl Query {
    pub fn new() -> Self {
        Query {
            years: None,
            set_range: None,
        }
    }

    pub fn set_years(&mut self, years: Vec<u16>) {
        let _ = self.years.take();
        self.years = Some(years);
    }

    pub fn set_all_years(&mut self) {
        let all_years: Vec<u16> = (MIN_YEAR_BRICK_ECONOMY..=current_year()).collect();
        self.set_years(all_years);
    }

    pub fn change_set_range(&mut self, set_numbers: Vec<u32>) {
        let _ = self.set_range.take();
        self.set_range = Some(set_numbers);
    }
}
