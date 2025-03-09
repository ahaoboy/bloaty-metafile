use bloaty_metafile::from_csv;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "BINARY")]
    pub name: String,

    #[arg(short, long)]
    pub lock: Option<String>,

    #[arg(short, long, default_value = "0")]
    pub deep: usize,

    #[arg(short, long, default_value = "false")]
    pub no_sections: bool,

    #[arg()]
    pub path: Option<String>,
}

fn main() {
    let Args {
        name,
        lock,
        deep,
        path,
        no_sections,
    } = Args::parse();
    let csv = if let Some(path) = path {
        std::fs::read_to_string(path).expect("failed to read csv file")
    } else {
        std::io::read_to_string(std::io::stdin()).expect("failed to read csv from stdio")
    };
    let meta = from_csv(&csv, &name, lock, deep, no_sections);
    let s = serde_json::to_string(&meta).expect("failed to serde metafile to json");
    println!("{s}",);
}
