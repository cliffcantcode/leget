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

    pub fn change_set_range(&mut self, set_numbers: Vec<u32>) {
        let _ = self.set_range.take();
        self.set_range = Some(set_numbers);
    }
}
