bloaty-metafile is a cli tool to convert csv files generated by [bloaty](https://github.com/google/bloaty) to esbuild's [metafile](https://esbuild.github.io/api/#metafile) format, so that you can use [online tools](https://esbuild.github.io/analyze/) to analyze the size of the program

```bash
cargo binstall bloaty-metafile

# or instal from github
cargo install --git https://github.com/ahaoboy/bloaty-metafile

bloaty ./bloaty -d sections,symbols -n 0  --csv | bloaty-metafile > meta.json

bloaty ./target/release/bloaty-metafile -d sections,symbols -n 0  --csv | bloaty-metafile --name=bloaty-metafile --lock=Cargo.lock  > meta.json
```

## profile

In order for bloaty to parse symbol information properly, it is recommended to keep debug information and turn off lto and strip

```toml
debug = true
lto = false
strip = false
```

## csv format

Please make sure bloaty generates a csv file in the following format. If the program is too large and the generated json exceeds 100mb, use the -n parameter to reduce the amount of data.

```
sections,symbols,vmsize,filesize
.text,ossl_aes_gcm_encrypt_avx512,337642,337642
.text,ossl_aes_gcm_decrypt_avx512,337638,337638
```

## Esbuild Bundle Size Analyzer

https://esbuild.github.io/analyze/

## Usage

### lock file

Because bloaty's output does not include crate dependency information, the sizes of crates are all displayed separately.

![llrt-no-lock](https://github.com/user-attachments/assets/669c033f-72e8-49e9-b030-dffc370b6580)

If a lock file can be provided, by default, the Cargo.lock file in the current directory is used. the dependency size can be correctly displayed by analyzing the crate dependencies.

![llrt-lock](https://github.com/user-attachments/assets/756bb69e-d8b5-42b2-946f-8e5439284209)

### deep

For large applications, the dependency tree will be very deep, which will cause the generated JSON to be very large and contain too much useless information. You can use the --deep option to limit the maximum depth of the dependency.

The default value of deep is 8, which is probably suitable for most programs. 0 means no limit

deep: 4, json: 6.7M
![llrt-deep-4](https://github.com/user-attachments/assets/2780c0ff-3a04-4aa3-946f-5c024347f1dd)

deep: 8, json: 12M
![llrt-deep-8](https://github.com/user-attachments/assets/89a786ff-45e6-47b7-a931-edd59d1dff30)


## windows

bloaty: PE doesn't support this data source

bloaty-metafile just converts the csv output by bloaty to json. You can generate csv files on other platforms with bloaty, and then convert them on windows.
