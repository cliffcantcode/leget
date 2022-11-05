//! Command line parsing and logic

use crate::query::Query;
use crate::scraper_utils::{make_selector, throttle};
use crate::set_data::SetData;

use clap::Parser;
use lazy_static::lazy_static;
use polars::prelude::*;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs::File;

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
    // these literally says 'placeholder' so this might break
    static ref PRICE_ROWS_SELECTOR: Selector = make_selector("#ContentPlaceHolder1_PanelSetPricing div.row");
    // value is nested under a hover
    static ref SPAN_HELPPOPOVER: Selector = make_selector("span.helppopover");

    // create regular expressions
    // if there is no ',' then the regex fails to find a second "set" of digits
    static ref RE_NUMBER_THEN_AMPERSAND: Regex = Regex::new(r"(\d+,?\d?+)&?").expect("A Regex of a number before an '&'.");
    static ref RE_DOLLARS: Regex = Regex::new(r"\$(\d?+,?\d?+\.\d?+)").expect("A Regex of a dollar amount after the '$'.");
    static ref RE_YEAR: Regex = Regex::new(r"[\s>](\d{4})[<\s]").expect("A Regex for a 4 digit number.");
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Leget {
    // try to limit inputs to just valid years
    /// the year made of sets you want to scan for. e.g. 2020 2021 2022 etc.
    #[arg(value_parser = clap::value_parser!(u16).range(1949..2200))]
    #[arg(short, long, group="year", num_args=1..100)]
    years: Option<Vec<u16>>,

    /// scan full range of years from 1949 (oldest on brickeconomy)
    #[arg(long, group = "year")]
    all_years: bool,

    // TODO: default the reading of set_number_range from our csv
    /// opt out of using the stored set_list.csv which enabled by default
    #[arg(long)]
    skip_set_list: bool,

    // TODO: assert somewhere that the second must be > than the first
    // TODO: could probably shorten this name
    /// scrape by set number. you must give a range
    #[arg(short, long, group = "set_range", num_args = 2)]
    set_number_range: Option<Vec<u32>>,

    // TODO: this might want to be a subcommand
    /// use this to update the file that lists valid sets
    #[arg(short = 'S', long, group = "set_range", num_args = 2)]
    update_set_list: Option<Vec<u32>>,
}

impl Leget {
    pub async fn exec(mut self) -> color_eyre::Result<()> {
        let mut query = Query::new();
        let mut set_data = SetData::new();

        // We can scrape the site once our query settings are ready
        let client = reqwest::Client::new();

        // Read in stored list of sets
        // append doesn't work if dtypes are mismatched; defaults are mismatched on read of csv
        let mut set_list_schema = Schema::new();
        set_list_schema.with_column("set_number".to_string(), DataType::Utf8);
        set_list_schema.with_column("year".to_string(), DataType::Utf8);
        set_list_schema.with_column("pieces".to_string(), DataType::Float32);

        if self.all_years {
            query.set_all_years();
        }

        if let Some(years) = self.years {
            query.set_years(years);
        }

        if query.years.is_some() {
            println!(
                "{:?}",
                query.years.as_ref().expect("A reference to query.years")
            );
        }

        // needs to be before we replace set_number_range with update_set_list
        if let Some(ref range) = self.set_number_range {
            query.set_set_number_range(range.to_vec());
        }

        // swap values for set list before update_set_list so they aren't clashing
        // TODO: the names here are super confusing
        if !self.skip_set_list {
            // read in the set list
            let set_list_lf: LazyFrame = CsvReader::from_path("set_list.csv")
                .expect("A reader connection to set_list.csv")
                .with_dtypes(Some(&set_list_schema))
                .has_header(true)
                .finish()
                .expect("A polars DataFrame from set_list.csv")
                .lazy();

            // gather set range into a vec so we can make a df
            if let Some(ref range) = query.set_number_range {
                let mut filter_sets: Vec<String> = vec![];
                for set in range[0]..=range[1] {
                    let mut set: String = set.to_string();
                    set.push_str("-1");
                    filter_sets.push(set);
                }

                let filter_sets_lf: LazyFrame = df! {
                    "sets_to_filter" => filter_sets,
                }
                .expect("A DataFrame of my sets to filter to.")
                .lazy();

                let joined_lf = set_list_lf.inner_join(
                    filter_sets_lf,
                    col("set_number"),
                    col("sets_to_filter"),
                );
                let df = joined_lf.collect().expect("The filtered df.");
                println!("joined DF: {:?}", &df);

                // TODO: use this on actual results
                let valid_sets: Vec<&str> = df
                    .column("set_number")
                    .expect("The Series of set_numbers.")
                    .utf8()
                    .expect("Parsed Series into Utf8.")
                    .into_no_null_iter()
                    .collect();
                println!("valid_sets: {:?}", &valid_sets);
            }
        }

        // if update_set_list is set we need to swap set range and create a flag.
        let mut update_set_list_flag = false;
        if let Some(ref range) = self.update_set_list {
            // you shouldn't use the set list to update itself.
            if !self.skip_set_list {
                println!("Overriding --skip-set-list=false. You should not use the set list to update itself.");
                self.skip_set_list = true;
            }
            query.set_set_number_range(range.to_vec());

            update_set_list_flag = true;
        }

        // Scrape by set numbers
        if let Some(range) = query.set_number_range {
            for set_number in range[0]..=range[1] {
                let url = format!(
                    "https://www.brickeconomy.com/set/{number}-1/",
                    number = set_number
                );

                // TODO: is there a way to get this to play nice with async?
                throttle();
                let response = client.get(url).send().await.expect("An async get request.");

                match response.status() {
                    reqwest::StatusCode::OK => {
                        let content = response
                            .text()
                            .await
                            .expect("The text of the get response.");
                        let document = Html::parse_document(&content);

                        let set_details = document.select(&SET_DETAILS);

                        // Catch all other edge cases so that the columns are the same len
                        // sometimes the header isn't even there, not sure if forcing it is the best
                        if set_data.set_number.len() > set_data.pieces.len() {
                            set_data.pieces.push(None);
                        }
                        assert_eq!(
                            &set_data.set_number.len(),
                            &set_data.pieces.len(),
                            "Set number and pieces columns aren't the same length after set #{:?}.",
                            set_data
                                .set_number
                                .last()
                                .expect("The last value of set_data.set_number.")
                        );
                        // sometimes there just isn't a place to get the retail price
                        if set_data.set_number.len() > set_data.retail_price.len() {
                            set_data.retail_price.push(None);
                        }
                        assert_eq!(
                            &set_data.set_number.len(),
                            &set_data.retail_price.len(),
                            "Set number and retail_price columns aren't the same length after set #{:?}.",
                            set_data
                                .set_number
                                .last()
                                .expect("The last value of set_data.set_number.")
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
                                        "Set number" => set_data.set_number.push(
                                            item.next()
                                                .expect("The next set number from set details.")
                                                .inner_html(),
                                        ),
                                        "Name" => {
                                            set_data.name.push(
                                                item.next()
                                                    .expect("The next name from set details.")
                                                    .inner_html(),
                                            );
                                        }
                                        "Year" => {
                                            if let Some(year) = item.next() {
                                                let year = year.inner_html();
                                                let numbers = RE_YEAR.captures(&year);
                                                let numbers = numbers.expect(
                                                    "The matches of a regex for 4 digit numbers.",
                                                );
                                                let year = &numbers[1];
                                                set_data.year.push(Some(year.to_string()));
                                            } else {
                                                set_data.year.push(None);
                                            }
                                        }
                                        "Pieces" => {
                                            if let Some(pieces) = item.next() {
                                                let piece_count = pieces.inner_html();
                                                let numbers =
                                                    RE_NUMBER_THEN_AMPERSAND.captures(&piece_count);
                                                let numbers = numbers.expect("The matches of a regex with a number before an '&'.");
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
                                        let mut listed_price =
                                            document.select(&TABLE_TR_TD_DIV_SPAN_A);
                                        if let Some(price) = listed_price.next() {
                                            let price = price.inner_html();
                                            let price = RE_DOLLARS.captures(&price).expect(
                                                "The matches of a regex with a number after a '$'.",
                                            );
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
                                                let value_headers =
                                                    header.select(&SPAN_HELPPOPOVER);
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
                                                                let price =
                                                                    RE_DOLLARS.captures(&price);
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
                    set_data
                        .set_number
                        .last()
                        .expect("The last value in set_data.set_number.")
                );
            }
        }

        // TODO: make this iterate through all years in query
        if let Some(years_vec) = query.years {
            let url = format!(
                "https://www.brickeconomy.com/sets/year/{year}",
                year = years_vec[0]
            );

            let response = client.get(url).send().await.expect("An async get request.");

            match response.status() {
                reqwest::StatusCode::OK => {
                    let content = response
                        .text()
                        .await
                        .expect("The text response of the year's get request.");
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
            "Set number and name columns aren't the same length."
        );
        assert_eq!(
            &set_data.set_number.len(),
            &set_data.year.len(),
            "Set number and year columns aren't the same length."
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
        let s_year = Series::new("year", &set_data.year);
        let s_retail_price = Series::new("retail_price", &set_data.retail_price);
        let s_value = Series::new("value", &set_data.value);
        let s_listed_price = Series::new("listed_price", &set_data.listed_price);
        let s_pieces = Series::new("pieces", &set_data.pieces);

        // do everything else, but control the output
        if update_set_list_flag {
            let mut df: DataFrame =
                DataFrame::new(vec![s_set_number, s_year, s_pieces]).expect("A Polars DataFrame.");

            let mut lf: LazyFrame = df.clone().lazy();
            lf = lf
                // greater than covers nulls
                .filter(col("pieces").gt(1));

            df = lf
                .collect()
                .expect("An executed LazyFrame for scanned sets.");
            println!("{}", &df);

            // read in the set list
            let mut set_list_df: DataFrame = CsvReader::from_path("set_list.csv")
                .expect("A reader connection to set_list.csv")
                .with_dtypes(Some(&set_list_schema))
                .has_header(true)
                .finish()
                .expect("A polars DataFrame from set_list.csv");

            set_list_df
                .extend(&df)
                .expect("The scanned df appended to the set_list_df.");
            set_list_df = set_list_df
                .unique(Some(&["set_number".to_string()]), UniqueKeepStrategy::First)
                .expect("A DataFrame with no duplicate set numbers.")
                .sort(["set_number"], false)
                .expect("A asc sorted DataFrame by set number.");
            println!("set_list_df: {}", &set_list_df);

            let set_list = File::create("set_list.csv").expect("The creation of the set_list.csv");
            let mut writer: CsvWriter<File> = CsvWriter::new(set_list).has_header(true);
            writer
                .finish(&mut set_list_df)
                .expect("The writting of our data to set_list.csv");
        } else {
            // TODO: DRY
            let df: PolarsResult<DataFrame> = DataFrame::new(vec![
                s_set_number,
                s_name,
                s_year,
                s_retail_price,
                s_value,
                s_listed_price,
                s_pieces,
            ]);

            let lf: LazyFrame = df.expect("A Polars DataFrame.").lazy();
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
            let mut lf = lf.collect().expect("An executed LazyFrame.");
            println!("{}", &lf);

            let legot_csv = File::create("legot.csv").expect("The creation of the legot.csv");
            let mut writer: CsvWriter<File> = CsvWriter::new(legot_csv).has_header(true);
            writer
                .finish(&mut lf)
                .expect("The writting of our data to legot.csv");
        }

        Ok(())
    }
}
