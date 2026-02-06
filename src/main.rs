use clap::{Parser, Subcommand};
use std::fs::File;
use std::time::Instant;

mod schema;
mod format;
mod deinflect;

use format::convert_yomitan_dictionary;
use format::Dictionary;
use format::types::QueryKindKey;
use format::container::{ContainerFileInfo, ContainerHeader, Role, open_container, allow_dev_version};

type CliResult = Result<(), Box<dyn std::error::Error>>;

// TODO: API that is simple "jp string to HTML" interface

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
        Ok(convert_yomitan_dictionary(
            &self.input,
            &self.output,
            ContainerHeader::new(
                self.name.clone(),
                self.revision_name.clone(),
                self.revision,
                Role::Dictionary,
                0,
            ),
        )?)
    }
}

#[derive(Parser)]
pub struct LookupCommand {
    pub word: String,

    #[arg(required = true)]
    pub dictionaries: Vec<String>,

    #[arg(long = "deinflection-rules")]
    pub deinflection_rules: Option<String>,
}

impl Execute for LookupCommand {
    fn execute(&self) -> CliResult {
        // TODO: just .map, more concise, need to incl store though
        let dicts: Vec<Dictionary> = self.dictionaries
            .iter()
            .map(|path| open_container::<Dictionary>(path, true))
            .collect::<Result<_, _>>()?;

        let mut terms = Vec::new();

        if let Some(deinflection_rules) = &self.deinflection_rules {
            let json_content = std::fs::read_to_string(deinflection_rules)?;

            let transforms = deinflect::TransformSet::from_json(&json_content)?;

            let deinflector = deinflect::Deinflector::make(transforms.clone())?;

            let start = Instant::now();
            let results = deinflector.deinflect(&self.word);
            for result in &results {
                terms.push(result.term.clone());
            }
            let elapsed = start.elapsed();

            println!("{:?} deinflection", elapsed);
            println!("{} terms from deinflection, {} post-filtering", results.len(), terms.len());
    
        } else {
            terms.push(self.word.clone());
        }

        let start = Instant::now();
        let mut result_count = 0;
        for dict in &dicts {
            for term in &terms {
                result_count += dict.index.lookup(QueryKindKey::Term, term).len();
            }
        }
        let elapsed = start.elapsed();

        println!("filtered term count {}", terms.len());
        println!("{:?} lookup", elapsed);
        println!("{} results", result_count);

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
        let file = File::open(self.path.clone())?;
        let container = ContainerFileInfo::read_container(&file)?;
        println!("{:#?}", container.header);

        match container.header.role {
            Role::Dictionary => {
                let dict = open_container::<Dictionary>(&self.path, true)?;

                // TODO: reimpl size stats
                // println!("\nFST size: {} KB", dict.index.len()/1024);
                // println!("\nStore size: {} KB", dict.index.data.len()/1024);

                let key_kinds = [
                    (QueryKindKey::Term, "term"),
                    (QueryKindKey::Kanji, "kanji"),
                    (QueryKindKey::Tag, "tag"),
                    (QueryKindKey::TermMeta, "term meta"),
                    (QueryKindKey::KanjiMeta, "kanji meta"),
                    (QueryKindKey::File, "file"),
                ];

                for key_kind in &key_kinds {
                    println!("  {} {} entries", dict.index.keys(key_kind.0).len(), key_kind.1);
                }

                // TODO: per key information about store using compressed size
                // need to get all keys, sort by offset, then extract size between keys
            }
            _ => {}
        }

        Ok(())
    }
}

fn main() {
    allow_dev_version(std::env::var("TOMOCHAN_DEV")
        .map(|v| v.len() > 0)
        .unwrap_or(false));

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