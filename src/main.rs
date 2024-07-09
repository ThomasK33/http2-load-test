use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Server address in the format hostname:port
    address: String,

    /// Target request rate (requests per second)
    #[arg(short, long, default_value_t = 1.0)]
    rate: f32,

    /// Total number of requests to execute
    #[arg(short, long, default_value_t = 1.0)]
    total: f32,
}

fn main() {
    let cli = Cli::parse();

    println!("Address {}", cli.address);
}
