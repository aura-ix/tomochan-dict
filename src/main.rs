use std::env;
use std::process;
use std::time::{Duration, Instant};

mod schema;
mod format;

use schema::Dictionary;
use format::{DictionaryIndexBuilder, DictionaryLookup};

fn print_results<T: std::fmt::Debug>(results: &[T]) {
    if results.is_empty() {
        println!("No results found");
    } else {
        println!("{:#?}", results);
    }
}

fn print_optional<T: std::fmt::Debug>(result: &Option<T>) {
    if result.is_none() {
        println!("No results found");
    } else {
        println!("{:#?}", result);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }
    
    let result = match args[1].as_str() {
        "convert" if args.len() >= 4 => {
            convert_dictionary(&args[2], &args[3])
        }
        "lookup" if args.len() >= 5 => {
            lookup_command(&args[2], &args[3], &args[4])
        }
        "lookup-random" if args.len() >= 3 => {
            let count = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
            benchmark_random_lookups(&args[2], count)
        }
        "convert" => {
            eprintln!("Usage: {} convert <dictionary_directory> <output_file>", args[0]);
            process::exit(1);
        }
        "lookup" => {
            eprintln!("Usage: {} lookup <dictionaries> <type> <key>", args[0]);
            eprintln!("Types: term, kanji, tag, term-meta, kanji-meta, file");
            process::exit(1);
        }
        "lookup-random" => {
            eprintln!("Usage: {} lookup-random <dictionaries> [count]", args[0]);
            process::exit(1);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage(&args[0]);
            process::exit(1);
        }
    };
    
    match result {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            process::exit(1);
        }
    }
}

fn print_usage(program: &str) {
    eprintln!("Usage: {} <command> [args...]", program);
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  convert <dict_dir> <output_file>");
    eprintln!("      Convert a dictionary directory to a .yomidict file");
    eprintln!();
    eprintln!("  lookup <dictionaries> <type> <key>");
    eprintln!("      Look up a key in one or more dictionaries");
    eprintln!("      Types: term, kanji, tag, term-meta, kanji-meta, file");
    eprintln!("      Dictionaries: comma-separated .yomidict files");
    eprintln!();
    eprintln!("  lookup-random <dictionaries> [count]");
    eprintln!("      Benchmark random lookups (default count: 100)");
    eprintln!("      Dictionaries: comma-separated .yomidict files");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} convert ./jmdict ./jmdict.yomidict", program);
    eprintln!("  {} lookup ./jmdict.yomidict term 読む", program);
    eprintln!("  {} lookup ./d1.yomidict,./d2.yomidict term 読む", program);
    eprintln!("  {} lookup-random ./jmdict.yomidict 1000", program);
}

fn convert_dictionary(dict_dir: &str, output_file: &str) -> Result<(), String> {
    println!("Loading dictionary from: {}", dict_dir);
    let dict = Dictionary::from_directory(dict_dir)?;
    
    println!("Loaded:");
    println!("  Terms: {}", dict.terms.len());
    println!("  Kanji: {}", dict.kanji.len());
    println!("  Tags: {}", dict.tags.len());
    println!("  Term Meta: {}", dict.term_meta.len());
    println!("  Kanji Meta: {}", dict.kanji_meta.len());
    
    println!("\nBuilding indices...");
    let mut builder = DictionaryIndexBuilder::new()?;
    builder.set_source_directory(dict_dir.to_string());
    builder.import_dictionary(dict)?;
    
    println!("Finalizing and saving to single file...");
    builder.finalize_to_single_file(output_file)?;
    
    if let Ok(metadata) = std::fs::metadata(output_file) {
        let size_mb = metadata.len() as f64 / 1_048_576.0;
        println!("\nCreated: {} ({:.2} MB)", output_file, size_mb);
    } else {
        println!("\nCreated: {}", output_file);
    }
    
    Ok(())
}

fn lookup_command(dict_files: &str, lookup_type: &str, key: &str) -> Result<(), String> {
    let files: Vec<&str> = dict_files.split(',').map(|s| s.trim()).collect();
    
    println!("Loading {} dictionary/dictionaries...", files.len());
    
    let mut lookups: Vec<DictionaryLookup> = files
        .iter()
        .map(|file| DictionaryLookup::open_package(file))
        .collect::<Result<Vec<_>, _>>()?;
    
    for (idx, lookup) in lookups.iter_mut().enumerate() {
        if files.len() > 1 {
            println!("\n=== Dictionary {} ({}) ===", idx + 1, files[idx]);
        }
        
        match lookup_type {
            "term" => {
                let results = lookup.terms.lookup_terms_full(key);
                print_results(&results);
            }
            "kanji" => {
                let results = lookup.terms.lookup_kanji_full(key);
                print_results(&results);
            }
            "tag" => {
                let result = lookup.terms.lookup_tag(key);
                print_optional(&result);
            }
            "term-meta" => {
                let results = lookup.terms.lookup_term_meta_full(key);
                print_results(&results);
            }
            "kanji-meta" => {
                let results = lookup.terms.lookup_kanji_meta_full(key);
                print_results(&results);
            }
            "file" => {
                if let Some(offset) = lookup.terms.lookup_extra_file(key) {
                    let data = lookup.terms.get_extra_file(offset)?;
                    println!("File found: {} bytes", data.len());
                } else {
                    println!("File not found");
                }
            }
            _ => return Err(format!("Unknown lookup type: {}. Valid types: term, kanji, tag, term-meta, kanji-meta, file", lookup_type))
        }
    }
    
    Ok(())
}

fn benchmark_random_lookups(package_file: &str, count: usize) -> Result<(), String> {
    let package_files: Vec<&str> = package_file.split(',').map(|s| s.trim()).collect();
    
    println!("Loading {} dictionary/dictionaries...", package_files.len());
    for (i, file) in package_files.iter().enumerate() {
        println!("  {}: {}", i + 1, file);
    }
    
    let load_start = Instant::now();
    let mut lookups: Vec<DictionaryLookup> = package_files
        .iter()
        .map(|file| DictionaryLookup::open_package(file))
        .collect::<Result<Vec<_>, _>>()?;
    let load_time = load_start.elapsed();
    
    println!("\nLoaded {} dictionaries in {:?}", lookups.len(), load_time);
    if lookups.len() > 1 {
        println!("Avg load time: {:?}", load_time / lookups.len() as u32);
    }
    
    println!("Collecting all terms...");
    let collect_start = Instant::now();
    let mut all_terms: Vec<(usize, String)> = Vec::new();
    
    for (dict_idx, lookup) in lookups.iter().enumerate() {
        let keys = lookup.terms.term_keys();
        println!("  Dictionary {}: {} terms", dict_idx + 1, keys.len());
        for key in keys {
            all_terms.push((dict_idx, key));
        }
    }
    
    let collect_time = collect_start.elapsed();
    println!("Collected {} total terms in {:?}", all_terms.len(), collect_time);
    
    if all_terms.is_empty() {
        return Err("No terms found in dictionaries".to_string());
    }
    
    let lookup_count = count.min(all_terms.len());
    println!("\n=== Random Lookup Benchmark ===");
    println!("Performing {} lookups from {} total terms...", lookup_count, all_terms.len());
    
    let mut total_results = 0;
    let mut total_time = Duration::ZERO;
    
    for i in 0..lookup_count {
        let idx = (i * 7919) % all_terms.len();
        let (dict_idx, term) = &all_terms[idx];
        
        let start = Instant::now();
        let results = lookups[*dict_idx].terms.lookup_terms_full(term);
        let elapsed = start.elapsed();
        
        total_results += results.len();
        total_time += elapsed;
        
        if i < 5 {
            if lookups.len() > 1 {
                println!("  {}: [Dict {}] \"{}\" - {} result(s) in {:?}", 
                    i + 1, dict_idx + 1, term, results.len(), elapsed);
            } else {
                println!("  {}: \"{}\" - {} result(s) in {:?}", 
                    i + 1, term, results.len(), elapsed);
            }
        }
    }
    
    if lookup_count > 5 {
        println!("  ... {} more lookups", lookup_count - 5);
    }
    
    print_benchmark_stats(lookup_count, total_results, total_time);
    
    Ok(())
}

fn print_benchmark_stats(lookup_count: usize, total_results: usize, total_time: Duration) {
    let avg_time = total_time / lookup_count as u32;
    let lookups_per_sec = lookup_count as f64 / total_time.as_secs_f64();
    let avg_results = total_results as f64 / lookup_count as f64;
    
    println!("\n=== Benchmark Results ===");
    println!("Total lookups:        {}", lookup_count);
    println!("Total results:        {}", total_results);
    println!("Avg results/lookup:   {:.2}", avg_results);
    println!("Total lookup time:    {:?}", total_time);
    println!("Avg time per lookup:  {:?}", avg_time);
    println!("Lookups per second:   {:.2}", lookups_per_sec);
    
    let avg_micros = total_time.as_micros() / lookup_count as u128;
    println!("Avg time:             {} us", avg_micros);
}