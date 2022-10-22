use chrono::offset::Utc;
use chrono::Datelike;
use clap::{Parser, Subcommand, ValueEnum};

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
    None,
    Current,
    All,
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
            Some(YearMode::None) => println!("None"),
            // get current year and push it to our query
            Some(YearMode::Current) => query.set_current_year(),
            Some(YearMode::All) => println!("All"),
            // default to current year
            None => {
                query.set_current_year();
            }
        },
    }

    println!("{:?}", query.years.unwrap()[0]);
}
