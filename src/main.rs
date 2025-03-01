use bloaty_metafile::from_csv;
fn main() {
    let csv = std::io::read_to_string(std::io::stdin()).expect("failed to read csv from stdio");
    let meta = from_csv(&csv);
    let s = serde_json::to_string(&meta).expect("failed to serde metafile to json");
    println!("{s}",);
}
