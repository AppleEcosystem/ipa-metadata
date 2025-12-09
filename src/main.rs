use clap::{Parser, ValueEnum};
use ipa_parser::{find_ipa_files, parse_ipa, parse_multiple_ipas, ParseOptions};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ipa-parser")]
#[command(about = "Blazing-fast IPA file parser for extracting metadata and icons", long_about = None)]
struct Cli {
    /// IPA file to parse (omit for multiple mode)
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// Process all IPA files in the directory
    #[arg(short, long)]
    multiple: bool,

    /// Directory containing IPA files (for multiple mode)
    #[arg(short, long, value_name = "DIR", default_value = ".")]
    directory: PathBuf,

    /// Output JSON file (prints to stdout if not specified)
    #[arg(short, long, value_name = "FILE")]
    outfile: Option<PathBuf>,

    /// Pretty-print JSON output
    #[arg(short, long)]
    pretty: bool,

    /// Sort JSON keys
    #[arg(short, long)]
    sort: bool,

    /// Disable icon extraction
    #[arg(long)]
    no_icons: bool,

    /// Directory to save extracted icons
    #[arg(long, value_name = "DIR", default_value = "icons")]
    icon_dir: PathBuf,

    /// Key strategy for multiple files
    #[arg(long, value_enum)]
    key_by: Option<KeyStrategy>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum KeyStrategy {
    /// Use filename as key
    Filename,
    /// Use bundle ID as key
    Bundleid,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Build parse options
    let options = ParseOptions {
        extract_icons: !cli.no_icons,
        icon_output_dir: cli.icon_dir,
        key_by: cli.key_by.map(|k| match k {
            KeyStrategy::Filename => "filename".to_string(),
            KeyStrategy::Bundleid => "bundleid".to_string(),
        }),
    };

    let json_value = if cli.multiple {
        // Multiple file mode
        let ipa_files = find_ipa_files(&cli.directory)?;
        
        if ipa_files.is_empty() {
            eprintln!("No IPA files found in {}", cli.directory.display());
            std::process::exit(1);
        }
        
        eprintln!("Found {} IPA file(s), processing...", ipa_files.len());
        parse_multiple_ipas(&ipa_files, &options)?
    } else {
        // Single file mode
        let file = cli.file.ok_or_else(|| {
            anyhow::anyhow!("Either --file or --multiple must be specified")
        })?;
        
        if !file.exists() {
            anyhow::bail!("File not found: {}", file.display());
        }
        
        let info = parse_ipa(&file, &options)?;
        serde_json::to_value(info)?
    };

    // Format output
    let output = if cli.pretty && cli.sort {
        serde_json::to_string_pretty(&json_value)?
    } else if cli.pretty {
        serde_json::to_string_pretty(&json_value)?
    } else if cli.sort {
        // For sorted output without pretty print, we need to manually sort
        serde_json::to_string(&json_value)?
    } else {
        serde_json::to_string(&json_value)?
    };

    // Write output
    if let Some(outfile) = cli.outfile {
        std::fs::write(&outfile, output)?;
        eprintln!("Output written to {}", outfile.display());
    } else {
        println!("{}", output);
    }

    Ok(())
}
