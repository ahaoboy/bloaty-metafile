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

    #[arg(long, default_value = "false")]
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
    // Check if JSON string is too large (JavaScript string length limit)
    // JavaScript max string length is 2^30 - 1 (0x3fffffff) characters
    // But V8 uses 0x1fffffe8 as practical limit
    const MAX_JSON_LENGTH: usize = 0x1fff_ffe8; // ~536MB

    let json_len = s.len();

    if json_len > MAX_JSON_LENGTH {
        eprintln!(
            "Warning: JSON output is too large ({} bytes, {} MB)",
            json_len,
            json_len >> 20
        );
        eprintln!("This exceeds JavaScript's maximum string length (0x1fffffe8 characters)");
        eprintln!("The output may not be usable in web-based tools like esbuild analyzer");
    }

    println!("{s}");
}
