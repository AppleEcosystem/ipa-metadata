# IPA Parser

Fast IPA metadata extractor written in Rust. I got tired of slow Python scripts, so I rewrote it.

## What it does

Extracts metadata from iOS IPA files - app name, version, bundle ID, file size, and the app icon. Also handles Apple's weird CgBI PNG format and converts it to normal PNG.

## Why Rust?

The original Python version was painfully slow when processing multiple files. This Rust version is way faster (like 20-100x depending on what you're doing) and uses way less memory.

## Building

You'll need Rust installed. Then:

```bash
cargo build --release
```

Binary ends up in `target/release/ipa-parser` (or `.exe` on Windows).

## Basic usage

Single file:
```bash
./target/release/ipa-parser --file myapp.ipa --pretty
```

Process a whole directory:
```bash
./target/release/ipa-parser --multiple --directory /path/to/ipas --pretty
```

Skip icon extraction if you just need metadata:
```bash
./target/release/ipa-parser --file myapp.ipa --no-icons
```

## Options

```
-f, --file <FILE>          Parse a single IPA file
-m, --multiple             Process all IPAs in a directory
-d, --directory <DIR>      Where to look for IPAs (default: current dir)
-o, --outfile <FILE>       Save JSON to file instead of stdout
-p, --pretty               Make the JSON readable
-s, --sort                 Sort the JSON keys
    --no-icons             Skip icon extraction (faster)
    --icon-dir <DIR>       Where to save icons (default: ./icons)
    --key-by <STRATEGY>    Use 'filename' or 'bundleid' as JSON keys
-h, --help                 Show help
```

## Output

Single file gives you something like:

```json
{
  "AppName": "MyApp",
  "AppVersion": "1.0.0",
  "AppBundleIdentifier": "com.example.myapp",
  "AppSize": 12345678,
  "IconName": "a1b2c3d4e5f6.png",
  "FileName": "myapp.ipa",
  "Timestamp": 1702123456
}
```

Multiple files with `--key-by filename`:

```json
{
  "myapp.ipa": {
    "AppName": "MyApp",
    "AppVersion": "1.0.0",
    "AppBundleIdentifier": "com.example.myapp",
    "AppSize": 12345678,
    "IconName": "a1b2c3d4e5f6.png",
    "Timestamp": 1702123456
  },
  "otherapp.ipa": {
    ...
  }
}
```

## Using as a library

Add to your `Cargo.toml`:
```toml
[dependencies]
ipa-parser = { path = "../path/to/this" }
```

Then:
```rust
use ipa_parser::{parse_ipa, ParseOptions};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ParseOptions {
        extract_icons: true,
        icon_output_dir: PathBuf::from("icons"),
        key_by: None,
    };
    
    let info = parse_ipa("app.ipa", &options)?;
    println!("App: {} v{}", info.app_name, info.app_version);
    
    Ok(())
}
```

## Performance notes

It's fast. Really fast. Processes most IPAs in under 100ms on decent hardware.

The parallel processing mode (`--multiple`) uses all your CPU cores, so it scales well. I've tested it with hundreds of IPAs and it handles them fine.

Memory usage is constant regardless of IPA size because it streams the ZIP contents instead of loading everything into RAM.

## How it works

### Icon extraction

Finds all PNG files matching the icon names from Info.plist, picks the largest one, normalizes it if it's in Apple's CgBI format, and saves it with an MD5-based filename.

### CgBI normalization

Apple's CgBI PNGs are basically regular PNGs with:
- A CgBI chunk before IHDR
- Premultiplied alpha
- RGB channels swapped (BGR instead of RGB)
- Different compression

The parser detects this and converts it back to standard PNG format that normal tools can read.

### Plist parsing

Handles both binary and XML plists automatically. Looks for the app name, version, bundle ID, and icon file names. Has fallbacks for different plist structures since Apple keeps changing things.

## Building with optimizations

For maximum performance, build with native CPU optimizations:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Or use the included script:
```bash
chmod +x build-optimized.sh
./build-optimized.sh
```

This enables CPU-specific instructions (SSE, AVX, etc.) which makes it even faster.

## Known issues

None that I know of, but if you find any, let me know.

## License

MIT - do whatever you want with it.