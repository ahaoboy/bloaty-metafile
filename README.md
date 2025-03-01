```bash
cargo binstall bloaty-metafile

cargo install --git https://github.com/ahaoboy/bloaty-metafile

bloaty ./bloaty -d sections,symbols -n 0  --csv | bloaty-metafile > meta.json

bloaty ./target/release/bloaty-metafile -d sections,symbols -n 0  --csv | bloaty-metafile --name=bloaty-metafile --lock=Cargo.lock  > meta.json
```

## csv format

Please make sure bloaty generates a csv file in the following format. If the
program is too large and the generated json exceeds 100mb, use the -n parameter
to reduce the amount of data.

```
sections,symbols,vmsize,filesize
.text,ossl_aes_gcm_encrypt_avx512,337642,337642
.text,ossl_aes_gcm_decrypt_avx512,337638,337638
```

## Esbuild Bundle Size Analyzer

https://esbuild.github.io/analyze/

## lock file

Because bloaty's output does not include crate dependency information, the sizes
of dependencies such as regex/regex_automata/regex_syntax are all displayed
separately.

![image](https://github.com/user-attachments/assets/802e39bd-8f2e-4929-b37d-bff4fcb641f2)

If a lock file can be provided, by default, the Cargo.lock file in the current
directory is used. the dependency size can be correctly displayed by analyzing
the crate dependencies.

![image](https://github.com/user-attachments/assets/96a403fb-c004-40c5-9f72-af44e2ea7213)
