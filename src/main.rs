use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    hello: Option<String>
}

fn main() {
    let cli = Cli::parse();

    // You can check the value provided by positional arguments, or options arguments
    if let Some(hello) = cli.hello.as_deref() {
        println!("Hello {}", hello);
    }
}
