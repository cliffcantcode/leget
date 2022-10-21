use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_enum)]
    year: YearMode,
}

#[derive(Clone, ValueEnum)]
enum YearMode {
    None,
    Current,
    All,
}

fn main() {
    let cli = Cli::parse();

    match cli.year {
        YearMode::None => println!("None"),
        YearMode::Current => println!("{}", 2022),
        YearMode::All => println!("All"),
    }
}
