```bash

bloaty ./bloaty -d sections,symbols -n 0  --csv | bloaty-metafile > meta.json

bloaty ./target/release/bloaty-metafile -d sections,symbols -n 0  --csv | bloaty-metafile --name=bloaty-metafile --cargo-lock=Cargo.lock  > meta.json

```

## Esbuild Bundle Size Analyzer

https://esbuild.github.io/analyze/