use chrono::offset::Utc;
use chrono::Datelike;
use clap::Parser;

const MIN_YEAR_BRICK_ECONOMY: u16 = 1949;

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
}

struct Query {
    years: Option<Vec<u16>>,
}

impl Query {
    fn new() -> Self {
        Query { years: None }
    }

    fn set_years(&mut self, years: Vec<u16>) {
        let _ = self.years.take();
        self.years = Some(years);
    }

    fn set_all_years(&mut self) {
        let all_years: Vec<u16> = (MIN_YEAR_BRICK_ECONOMY..=current_year()).collect();
        self.set_years(all_years);
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

    // We can scrape the site once our query settings are ready
    let client = reqwest::Client::new();

    // TODO: make this iterate through all years in query
    if let Some(years_vec) = query.years {
        let url = format!(
            "https://www.brickeconomy.com/sets/year/{year}",
            year = years_vec[0]
        );

        let response = client.get(url).send().await.unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                println!("{:?}", response);
            }
            problem => {
                panic!("There was a problem: {:?}", problem);
            }
        };
    }
}
