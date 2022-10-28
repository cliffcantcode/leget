mod scraper_utils;
use chrono::offset::Utc;
use chrono::Datelike;
use clap::Parser;
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{Html, Selector};

const MIN_YEAR_BRICK_ECONOMY: u16 = 1949;

// Convience function to avoid unwrap()ing all the time
fn make_selector(selector: &str) -> Selector {
    Selector::parse(selector).unwrap()
}

lazy_static! {
    // create selectors
    static ref TABLE: Selector = make_selector("table");
    static ref TR: Selector = make_selector("tr");
    static ref TD: Selector = make_selector("td");
    static ref H4: Selector = make_selector("h4");

    // create regular expressions
    static ref RE_NUMBER_THEN_AMPERSAND: Regex = Regex::new(r"(\d+,?\d+)&").unwrap();
    static ref RE_DOLLARS: Regex = Regex::new(r"^\$(\d+\.?\d+)$").unwrap();
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // try to limit inputs to just valid years
    #[arg(value_parser = clap::value_parser!(u16).range(1949..2200))]
    #[arg(short, long, group="year", num_args=1..100)]
    years: Option<Vec<u16>>,

    // use full range of years from 1949 (oldest on brickeconomy)
    #[arg(long, group = "year")]
    all_years: bool,

    // scrape by set number
    #[arg(short, long, num_args = 2)]
    set_number_range: Option<Vec<u32>>,
}

struct SetData {
    // can't be a number because it's formatted with a '-'
    set_number: Vec<String>,

    name: Vec<String>,

    listed_price: Vec<Option<f32>>,

    // u16 (65_535) since current largest set is only 11_695 pieces
    pieces: Vec<Option<u16>>,
}

impl SetData {
    fn new() -> Self {
        SetData {
            set_number: vec![],
            name: vec![],
            listed_price: vec![],
            pieces: vec![],
        }
    }
}

struct Query {
    years: Option<Vec<u16>>,
    set_number_range: Option<Vec<u32>>,
}

impl Query {
    fn new() -> Self {
        Query {
            years: None,
            set_number_range: None,
        }
    }

    fn set_years(&mut self, years: Vec<u16>) {
        let _ = self.years.take();
        self.years = Some(years);
    }

    fn set_all_years(&mut self) {
        let all_years: Vec<u16> = (MIN_YEAR_BRICK_ECONOMY..=current_year()).collect();
        self.set_years(all_years);
    }

    fn set_set_number_range(&mut self, set_numbers: Vec<u32>) {
        let _ = self.set_number_range.take();
        self.set_number_range = Some(set_numbers);
    }
}

fn current_year() -> u16 {
    let date = Utc::today();
    date.year().try_into().unwrap()
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut query = Query::new();
    let mut set_data = SetData::new();

    if cli.all_years {
        query.set_all_years();
    }

    if let Some(years) = cli.years {
        query.set_years(years);
    }

    if query.years.is_some() {
        println!("{:?}", query.years.as_ref().unwrap());
    }

    if let Some(set_numbers) = cli.set_number_range {
        query.set_set_number_range(set_numbers);
    }

    if query.set_number_range.is_some() {
        println!("{:?}", query.set_number_range.as_ref().unwrap());
    }

    // We can scrape the site once our query settings are ready
    let client = reqwest::Client::new();

    // Scrape by set numbers
    if let Some(range) = query.set_number_range {
        for set_number in range[0]..=range[1] {
            let url = format!(
                "https://www.brickeconomy.com/set/{number}-1/",
                number = set_number
            );

            // TODO: is there a way to get this to play nice with async?
            scraper_utils::throttle();
            let response = client.get(url).send().await.unwrap();

            match response.status() {
                reqwest::StatusCode::OK => {
                    let content = response.text().await.unwrap();
                    let document = Html::parse_document(&content);
                    // TODO: a lot of these selectors should probably be static
                    let set_details_selector = Selector::parse("div#SetDetails div.row").unwrap();
                    let col_xs_5_selector = Selector::parse("div.col-xs-5").unwrap();
                    let col_xs_7_selector = Selector::parse("div.col-xs-7").unwrap();
                    let table_tr_td_h1_selector = Selector::parse("table tr td h1").unwrap();
                    let table_tr_td_div_span_a_selector =
                        Selector::parse("table#sales_region_table tr td div span.a").unwrap();
                    // TODO: should probably get this from the 'set details' part of the page
                    let mut h1 = document.select(&table_tr_td_h1_selector);
                    let mut listed_price = document.select(&table_tr_td_div_span_a_selector);
                    let set_details = document.select(&set_details_selector);

                    // only push other data if there is a name
                    // TODO: need to change this now that name comes from set details
                    if let Some(_name) = h1.next() {
                        // push one item at a time incase there are multiple
                        // push set number (as a string because of the '-')
                        for detail in set_details {
                            let mut header = detail.select(&col_xs_5_selector);
                            let mut item = detail.select(&col_xs_7_selector);

                            if let Some(header) = header.next() {
                                match header.inner_html().as_str() {
                                    "Set number" => {
                                        set_data.set_number.push(item.next().unwrap().inner_html())
                                    }
                                    "Name" => {
                                        set_data.name.push(item.next().unwrap().inner_html());
                                    }
                                    "Pieces" => {
                                        if let Some(pieces) = item.next() {
                                            let piece_count = pieces.inner_html();
                                            let numbers = RE_NUMBER_THEN_AMPERSAND
                                                .captures(&piece_count)
                                                .unwrap();
                                            let piece_count =
                                                numbers[1].split(',').collect::<String>();
                                            if let Ok(count) = piece_count.parse::<u16>() {
                                                set_data.pieces.push(Some(count));
                                            } else {
                                                set_data.pieces.push(None);
                                            }
                                        } else {
                                            set_data.pieces.push(None);
                                        }
                                    }
                                    _ => continue,
                                }
                            }
                        }

                        // push listed price
                        if let Some(price) = listed_price.next() {
                            let price = price.inner_html();
                            let price = RE_DOLLARS.captures(&price).unwrap();
                            if let Ok(price) = price[1].parse::<f32>() {
                                set_data.listed_price.push(Some(price));
                            } else {
                                set_data.listed_price.push(None);
                            }
                        } else {
                            set_data.listed_price.push(None);
                        }
                    }
                }
                problem => {
                    panic!("There was a problem: {:?}", problem);
                }
            }
        }
    }

    // TODO: make this iterate through all years in query
    if let Some(years_vec) = query.years {
        let url = format!(
            "https://www.brickeconomy.com/sets/year/{year}",
            year = years_vec[0]
        );

        let response = client.get(url).send().await.unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                let content = response.text().await.unwrap();
                let document = Html::parse_document(&content);
                let h4_a_selector = Selector::parse("h4 a").unwrap();
                let h4 = document.select(&h4_a_selector);
                for item in h4 {
                    println!("{}", item.inner_html());
                }
            }
            problem => {
                panic!("There was a problem: {:?}", problem);
            }
        };
    }

    // make sure the index len is the same before we make a dataframe
    assert_eq!(
        &set_data.set_number.len(),
        &set_data.name.len(),
        "Set number and name  columns aren't the same length."
    );
    assert_eq!(
        &set_data.set_number.len(),
        &set_data.listed_price.len(),
        "Name and listed price columns aren't the same length."
    );
    assert_eq!(
        &set_data.set_number.len(),
        &set_data.pieces.len(),
        "Name and pieces columns aren't the same length."
    );

    println!(
        "Set numbers: {:?}\nNames: {:?}\nListed Prices: {:?}\nPieces: {:?}",
        set_data.set_number, set_data.name, set_data.listed_price, set_data.pieces
    );
}
