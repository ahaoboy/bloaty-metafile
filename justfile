default:
  just --list

bloaty-build:
  cargo build --profile bloaty
bloaty-csv:
  bloaty ./target/bloaty/bloaty-metafile -d sections,symbols -n 0 --csv > meta.csv
bloaty-json:
  bloaty-metafile meta.csv > meta.json
bloaty: bloaty-build bloaty-csv bloaty-json

clippy:
  cargo clippy --fix --allow-dirty --allow-staged --all-targets
fmt:
  cargo fmt
check: fmt clippy
