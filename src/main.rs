use bloaty_metafile::from_csv;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "Binary")]
    pub name: String,

    #[arg(short, long)]
    pub lock: Option<String>,

    #[arg(short, long, default_value = "8")]
    pub deep: usize,
}

fn main() {
    let Args { name, lock, deep } = Args::parse();
    let csv = std::io::read_to_string(std::io::stdin()).expect("failed to read csv from stdio");
    let meta = from_csv(&csv, &name, lock, deep);
    let s = serde_json::to_string(&meta).expect("failed to serde metafile to json");
    println!("{s}",);
}
