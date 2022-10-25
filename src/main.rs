use chrono::offset::Utc;
use chrono::Datelike;
use clap::Parser;
use lazy_static::lazy_static;
use scraper::{Html, Selector};

const MIN_YEAR_BRICK_ECONOMY: u16 = 1949;

// Convience function to avoid unwrap()ing all the time
fn make_selector(selector: &str) -> Selector {
    Selector::parse(selector).unwrap()
}

lazy_static! {
    static ref TABLE: Selector = make_selector("table");
    static ref TR: Selector = make_selector("tr");
    static ref H4: Selector = make_selector("h4");
    static ref TD: Selector = make_selector("td");
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
    // TODO: left off here
    if let Some(_range) = query.set_number_range {}

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
}
