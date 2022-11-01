mod scraper_utils;
use chrono::offset::Utc;
use chrono::Datelike;
use clap::Parser;
use lazy_static::lazy_static;
use polars::prelude::*;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs::File;

const MIN_YEAR_BRICK_ECONOMY: u16 = 1949;

// Convience function to avoid unwrap()ing all the time
fn make_selector(selector: &str) -> Selector {
    Selector::parse(selector).expect("A Selector from the parsed &str.")
}

lazy_static! {
    // create selectors
    static ref TABLE: Selector = make_selector("table");
    static ref TR: Selector = make_selector("tr");
    static ref TD: Selector = make_selector("td");
    static ref H4: Selector = make_selector("h4");
    static ref H4_A: Selector = make_selector("h4 a");
    static ref SET_DETAILS: Selector = make_selector("div#SetDetails div.row");
    static ref COL_XS_5: Selector = make_selector("div.col-xs-5");
    static ref COL_XS_7: Selector = make_selector("div.col-xs-7");
    static ref TABLE_TR_TD_H1: Selector = make_selector("table tr td h1");
    // gets listed price
    static ref TABLE_TR_TD_DIV_SPAN_A: Selector = make_selector("table#sales_region_table tr td div span.a");
    // it literally says 'placeholder' so this might break
    static ref PRICE_ROWS_SELECTOR: Selector = make_selector("#ContentPlaceHolder1_PanelSetPricing div.row");

    // create regular expressions
    // if there is no ',' then the regex fails to find a second "set" of digits
    static ref RE_NUMBER_THEN_AMPERSAND: Regex = Regex::new(r"(\d+,?\d?+)&?").expect("A Regex of a number before an '&'.");
    static ref RE_DOLLARS: Regex = Regex::new(r"\$(\d?+,?\d?+\.\d?+)").expect("A Regex of a dollar amount after the '$'.");
    // I think I can delete this: static ref RE_DOLLARS: Regex = Regex::new(r"\$([0-9]?+,?[0-9]+.[0-9]+)").unwrap();
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // try to limit inputs to just valid years
    /// the year made of sets you want to scan for. e.g. 2020 2021 2022 etc.
    #[arg(value_parser = clap::value_parser!(u16).range(1949..2200))]
    #[arg(short, long, group="year", num_args=1..100)]
    years: Option<Vec<u16>>,

    /// scan full range of years from 1949 (oldest on brickeconomy)
    #[arg(long, group = "year")]
    all_years: bool,

    // TODO: assert somewhere that the second must be > than the first
    /// scrape by set number. you must give a range
    #[arg(short, long, num_args = 2)]
    set_number_range: Option<Vec<u32>>,
}

struct SetData {
    // can't be a number because it's formatted with a '-'
    set_number: Vec<String>,

    name: Vec<String>,

    retail_price: Vec<Option<f32>>,

    // either market price or brickeconomy estimate
    value: Vec<Option<f32>>,

    // a seller's price; should be cheapest but not guaranteed
    listed_price: Vec<Option<f32>>,

    // u16 (65_535) since current largest set is only 11_695 pieces
    // not u16 because polars::perlude::NamedFrom isn't impled for Vec<Option<u16>>?
    pieces: Vec<Option<f32>>,
}

impl SetData {
    fn new() -> Self {
        SetData {
            set_number: vec![],
            name: vec![],
            retail_price: vec![],
            value: vec![],
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
    date.year().try_into().expect("A u16 of the current year.")
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

                    let set_details = document.select(&SET_DETAILS);

                    // sometimes the header isn't even there, not sure if forcing it is the best
                    if set_data.set_number.len() > set_data.pieces.len() {
                        set_data.pieces.push(None);
                    }
                    assert_eq!(
                        &set_data.set_number.len(),
                        &set_data.pieces.len(),
                        "Set number and pieces columns aren't the same length after set #{:?}.",
                        set_data.set_number.last().unwrap()
                    );

                    // push one item at a time incase there are multiple
                    // push set number (as a string because of the '-')
                    if set_data.set_number.len() == set_data.name.len() {
                        for detail in set_details {
                            let mut header = detail.select(&COL_XS_5);
                            let mut item = detail.select(&COL_XS_7);

                            // sometimes a header is repeated; e.g. new and used Value
                            let mut value_header_count = 0;

                            if let Some(header) = header.next() {
                                let header = header.inner_html();
                                match header.as_str() {
                                    "Set number" => {
                                        set_data.set_number.push(item.next().unwrap().inner_html())
                                    }
                                    "Name" => {
                                        set_data.name.push(item.next().unwrap().inner_html());
                                    }
                                    "Pieces" => {
                                        if let Some(pieces) = item.next() {
                                            let piece_count = pieces.inner_html();
                                            let numbers =
                                                RE_NUMBER_THEN_AMPERSAND.captures(&piece_count);
                                            let numbers = numbers.unwrap();
                                            let piece_count =
                                                numbers[1].split(',').collect::<String>();
                                            if let Ok(count) = piece_count.parse::<f32>() {
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

                                // push other items only once per valid set number
                                if header.as_str() == "Set number" {
                                    // push listed price
                                    let mut listed_price = document.select(&TABLE_TR_TD_DIV_SPAN_A);
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

                                    // push prices
                                    let price_rows = document.select(&PRICE_ROWS_SELECTOR);
                                    for row in price_rows {
                                        let headers = row.select(&COL_XS_5);
                                        let mut items = row.select(&COL_XS_7);

                                        for header in headers {
                                            let mut header_html = header.inner_html();
                                            let item = items.next();

                                            // some headers are further nested
                                            let value_selector =
                                                Selector::parse("span.helppopover").unwrap();
                                            let value_headers = header.select(&value_selector);
                                            for header in value_headers {
                                                header_html = header.inner_html();
                                            }

                                            match header_html.as_str() {
                                                "Retail price" => {
                                                    if let Some(price) = item {
                                                        let price = price.inner_html();
                                                        let price = RE_DOLLARS.captures(&price);
                                                        if let Some(price) = price {
                                                            if let Ok(price) =
                                                                price[1].parse::<f32>()
                                                            {
                                                                set_data
                                                                    .retail_price
                                                                    .push(Some(price));
                                                            }
                                                        } else {
                                                            set_data.retail_price.push(None);
                                                        }
                                                    } else {
                                                        set_data.retail_price.push(None);
                                                    }
                                                }
                                                // as either market price or brickeconomy estimate
                                                // depending if the set is still availible at retail
                                                "Value" | "Market price" => {
                                                    // sometimes there are both new and used
                                                    // values; new seems to be first
                                                    value_header_count += 1;
                                                    if value_header_count == 1 {
                                                        if let Some(price) = item {
                                                            // not using inner html since sometimes
                                                            // there is an additional <b> nested
                                                            let price = price.html();
                                                            let price = RE_DOLLARS.captures(&price);
                                                            if let Some(price) = price {
                                                                // regex doesn't handle ',' from
                                                                // numbers that get into the
                                                                // thousands
                                                                let price = price[1]
                                                                    .split(',')
                                                                    .collect::<String>();
                                                                if let Ok(price) =
                                                                    price.parse::<f32>()
                                                                {
                                                                    set_data
                                                                        .value
                                                                        .push(Some(price));
                                                                }
                                                            } else {
                                                                set_data.value.push(None);
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                problem => {
                    panic!("There was a problem: {:?}", problem);
                }
            }
            // need to catch if the html is being missed somewhere
            assert_eq!(
                &set_data.set_number.len(),
                &set_data.value.len(),
                "Set number and value columns aren't the same length after set #{:?}.",
                set_data.set_number.last().unwrap()
            );
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
                let h4 = document.select(&H4_A);
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
        &set_data.retail_price.len(),
        "Set number and retail price columns aren't the same length."
    );
    assert_eq!(
        &set_data.set_number.len(),
        &set_data.value.len(),
        "Set number and value columns aren't the same length."
    );
    assert_eq!(
        &set_data.set_number.len(),
        &set_data.listed_price.len(),
        "Set number and listed price columns aren't the same length."
    );
    assert_eq!(
        &set_data.set_number.len(),
        &set_data.pieces.len(),
        "Set number and pieces columns aren't the same length."
    );

    let s_set_number = Series::new("set_number", &set_data.set_number);
    let s_name = Series::new("name", &set_data.name);
    let s_retail_price = Series::new("retail_price", &set_data.retail_price);
    let s_value = Series::new("value", &set_data.value);
    let s_listed_price = Series::new("listed_price", &set_data.listed_price);
    let s_pieces = Series::new("pieces", &set_data.pieces);

    let df: PolarsResult<DataFrame> = DataFrame::new(vec![
        s_set_number,
        s_name,
        s_retail_price,
        s_value,
        s_listed_price,
        s_pieces,
    ]);

    let lf: LazyFrame = df.unwrap().lazy();
    let lf = lf
        .filter(col("listed_price").is_not_null())
        .filter(col("value").is_not_null())
        // greater than covers nulls
        .filter(col("pieces").gt(1))
        .with_column(
            ((col("listed_price") - col("value")) / col("value"))
                .alias("percent_discount_from_value"),
        )
        // TODO: I would like to not be repeating myself here
        .with_column(
            ((col("listed_price") - col("value")) / (col("value") * col("pieces")))
                .alias("percent_discount_from_value_per_piece"),
        )
        .sort("percent_discount_from_value_per_piece", Default::default());
    let mut lf = lf.collect().unwrap();
    println!("{:?}\n {} Rows", lf, set_data.set_number.len());

    let legot_csv = File::create("outputs/legot.csv").unwrap();
    let mut writer: CsvWriter<File> = CsvWriter::new(legot_csv).has_header(true);
    writer.finish(&mut lf).unwrap();
}
