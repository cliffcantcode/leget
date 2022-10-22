use chrono::offset::Utc;
use chrono::Datelike;
use clap::{Parser, Subcommand, ValueEnum};

const MIN_YEAR_BRICK_ECONOMY: u16 = 1949;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// scrapes lego sets by year made
    Year { mode: Option<YearMode> },
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, ValueEnum)]
enum YearMode {
    All,
    Current,
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

    fn set_current_year(&mut self) {
        self.set_years(vec![current_year()]);
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

fn main() {
    let cli = Cli::parse();

    let mut query = Query::new();

    match &cli.command {
        Commands::Year { mode } => match mode {
            // get current year and push it to our query
            Some(YearMode::Current) => query.set_current_year(),
            // use full range of years from 1949 (oldest on brickeconomy)
            Some(YearMode::All) => query.set_all_years(),
            // default to current year
            None => {
                query.set_current_year();
            }
        },
    }

    if query.years.is_some() {
        println!("{:?}", query.years.unwrap());
    }
}
