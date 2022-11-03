// where the data we pull will be stored
pub struct SetData {
    // can't be a number because it's formatted with a '-'
    pub set_number: Vec<String>,

    pub name: Vec<String>,

    pub year: Vec<Option<String>>,

    pub retail_price: Vec<Option<f32>>,

    // either market price or brickeconomy estimate
    pub value: Vec<Option<f32>>,

    // a seller's price; should be cheapest but not guaranteed
    pub listed_price: Vec<Option<f32>>,

    // u16 (65_535) since current largest set is only 11_695 pieces
    // not u16 because polars::perlude::NamedFrom isn't impled for Vec<Option<u16>>?
    pub pieces: Vec<Option<f32>>,
}

impl SetData {
    pub fn new() -> Self {
        SetData {
            set_number: vec![],
            name: vec![],
            year: vec![],
            retail_price: vec![],
            value: vec![],
            listed_price: vec![],
            pieces: vec![],
        }
    }
}
