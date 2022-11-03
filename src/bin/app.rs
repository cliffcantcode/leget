use clap::Parser;
use leget::Leget;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install().expect("color_eyre installing.");

    let app = Leget::parse();
    app.exec().await
}
