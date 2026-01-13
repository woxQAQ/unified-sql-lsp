use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/grammar/");

    let grammar_dir = Path::new("src/grammar");
    let out_dir = env::var("OUT_DIR").unwrap();

    // Check which dialects need regeneration (only if grammar files changed)
    let needs_regeneration = check_grammar_changes(grammar_dir);

    // Dialects to compile
    let dialects = vec![
        "base",
        "mysql-5.7",
        "mysql-8.0",
        "postgresql-12",
        "postgresql-14",
    ];

    // Phase 1: Generate all parser.c files first (can be parallelized)
    let mut parser_files: HashMap<String, String> = HashMap::new();

    for dialect in &dialects {
        let dialect_parser_c = grammar_dir.join(format!("gen/parser-{}.c", dialect));

        // Only regenerate if needed
        if needs_regeneration || !dialect_parser_c.exists() {
            println!("cargo:warning=Generating grammar for dialect: {}", dialect);

            unsafe {
                env::set_var("DIALECT", dialect);
            }

            let status = Command::new("tree-sitter")
                .args(["generate", "-o", "gen"])
                .current_dir(grammar_dir)
                .status();

            match status {
                Ok(s) if s.success() => {
                    let parser_c = grammar_dir.join("gen/parser.c");
                    if parser_c.exists() {
                        fs::rename(&parser_c, &dialect_parser_c)
                            .expect("Failed to rename parser.c");
                        parser_files.insert(
                            dialect.to_string(),
                            dialect_parser_c.to_string_lossy().to_string(),
                        );
                    }
                }
                Ok(s) => {
                    println!(
                        "cargo:warning=Failed to generate {} grammar: exit code {:?}",
                        dialect,
                        s.code()
                    );
                }
                Err(e) => {
                    println!(
                        "cargo:warning=Failed to run tree-sitter for {}: {}",
                        dialect, e
                    );
                    println!(
                        "cargo:warning=Install tree-sitter-cli: npm install -g tree-sitter-cli"
                    );
                }
            }
        } else {
            println!(
                "cargo:warning=Skipping generation for {} (already cached)",
                dialect
            );
            parser_files.insert(
                dialect.to_string(),
                dialect_parser_c.to_string_lossy().to_string(),
            );
        }
    }

    // Phase 2: Compile all parsers with cc (parallel at link level)
    let mut cc_build = cc::Build::new();
    cc_build.include(grammar_dir.join("gen"));

    for (dialect, parser_path) in &parser_files {
        let src_path = Path::new(parser_path);
        if src_path.exists() {
            let dest_path = Path::new(&out_dir).join(format!("parser-{}.c", dialect));
            fs::copy(src_path, &dest_path).expect("Failed to copy parser.c");
            cc_build.file(&dest_path);
        }
    }

    // Compile all together (faster than separate compilations)
    cc_build.compile("parsers");

    println!("cargo:warning=Compiled {} parsers", parser_files.len());

    println!("cargo:rerun-if-changed=src/grammar/grammar.js");
    println!("cargo:rerun-if-changed=src/grammar/dialect/");
}

fn check_grammar_changes(grammar_dir: &Path) -> bool {
    let gen_dir = grammar_dir.join("gen");
    if !gen_dir.exists() {
        return true;
    }

    // Check if any grammar file is newer than generated parsers
    let mut grammar_files = vec![grammar_dir.join("grammar.js")];

    // Add all dialect/*.js files
    if let Ok(entries) = fs::read_dir(grammar_dir.join("dialect")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "js").unwrap_or(false) {
                grammar_files.push(path);
            }
        }
    }

    // Get newest grammar file time
    let grammar_time = grammar_files
        .iter()
        .filter(|p| p.exists())
        .filter_map(|p| fs::metadata(p).ok().and_then(|m| m.modified().ok()))
        .max();

    // Get oldest parser file time - we want to know if ANY parser is older than grammar
    let parser_time = fs::read_dir(&gen_dir).ok().and_then(|entries| {
        entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                let name = path.to_string_lossy();
                path.extension().map(|e| e == "c").unwrap_or(false) &&
                    name.contains("parser-") &&
                    !name.contains("parser-mysql.c") &&  // Skip old files
                    !name.contains("parser-postgresql.c")
            })
            .filter_map(|e| fs::metadata(e.path()).ok().and_then(|m| m.modified().ok()))
            .min()
    });

    match (grammar_time, parser_time) {
        (Some(g), Some(p)) => g > p,
        _ => true,
    }
}
