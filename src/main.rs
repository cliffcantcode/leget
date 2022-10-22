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

// TODO: add query for scraping
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
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Year { mode } => match mode {
            Some(YearMode::None) => println!("None"),
            Some(YearMode::Current) => println!("{}", 2022),
            Some(YearMode::All) => println!("All"),
            None => {
                println!("{}", 2022);
            }
        },
    }

    let mut query = Query::new();
    query.set_years(vec![2022]);
    println!("{:?}", query.years.unwrap()[0]);
}
