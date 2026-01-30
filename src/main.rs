use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{Seek, SeekFrom};

mod schema;
mod format;
mod deinflect;

use format::{DictionaryPackage, convert_yomitan_dictionary};
use format::types::QueryKindKey;
use format::index::UnifiedFstIndex;
use format::store::UnifiedStore;
use format::container::{ContainerFileInfo, ContainerHeader, Role};

type CliResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Convert(ConvertCommand),
    Lookup(LookupCommand),
    Probe(ProbeCommand),
}

trait Execute {
    fn execute(&self) -> CliResult;
}

// TODO: autofill revision and revision_name with dev if not specified
// TODO: option to use current utc timestamp as revision?
#[derive(Parser)]
struct ConvertCommand {
    #[arg(long)]
    input: String,

    #[arg(long)]
    output: String,

    #[arg(long)]
    name: String,

    #[arg(long)]
    revision_name: String,

    #[arg(long)]
    revision: u64,
}

impl Execute for ConvertCommand {
    fn execute(&self) -> CliResult {
        let package = convert_yomitan_dictionary(&self.input)?;
        package.save(
            &self.output,
            ContainerHeader::new(
                self.name.clone(),
                self.revision_name.clone(),
                self.revision,
                Role::Dictionary,
                0,
            ),
        )?;

        Ok(())
    }
}

#[derive(Parser)]
struct LookupCommand {
    #[arg(long)]
    dict: String,

    #[arg(long)]
    kind: String,

    #[arg(long)]
    key: String,
}

impl Execute for LookupCommand {
    fn execute(&self) -> CliResult {
        // TODO

        Ok(())
    }
}

#[derive(Parser)]
struct ProbeCommand {
    #[arg(long)]
    path: String,
}

impl Execute for ProbeCommand {
    fn execute(&self) -> CliResult {
        let mut file = File::open(self.path.clone())?;
        let container = ContainerFileInfo::read_container(&file)?;
        println!("{:#?}", container.header);

        match container.header.role {
            Role::Dictionary => {
                file.seek(SeekFrom::Start(container.payload_offset))?;
                let package = DictionaryPackage::load_reader(&mut file)?;

                println!("\nFST size: {} KB", package.fst.len()/1024);
                let fst = UnifiedFstIndex::from_bytes(package.fst)?;

                let key_kinds = [
                    (QueryKindKey::Term, "term"),
                    (QueryKindKey::Kanji, "kanji"),
                    (QueryKindKey::Tag, "tag"),
                    (QueryKindKey::TermMeta, "term meta"),
                    (QueryKindKey::KanjiMeta, "kanji meta"),
                    (QueryKindKey::File, "file"),
                ];

                for key_kind in &key_kinds {
                    println!("  {} {} entries", fst.keys(key_kind.0).len(), key_kind.1);
                }

                println!("\nStore size: {} KB", package.data.len()/1024);
                // TODO: per key information about store using compressed size
                // need to get all keys, sort by offset, then extract size between keys
            }
            _ => {}
        }

        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Convert(args) => args.execute(),
        Commands::Lookup(args) => args.execute(),
        Commands::Probe(args) => args.execute(),
    };

    if let Err(err) = result {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
