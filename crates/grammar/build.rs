use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/grammar/");

    let grammar_dir = Path::new("src/grammar");
    let out_dir = env::var("OUT_DIR").unwrap();

    // Dialects to compile
    // Architecture: version-specific dialects are the base
    // - mysql-5.7 is the base MySQL dialect
    // - mysql-8.0 extends mysql-5.7
    // - postgresql-12 is the base PostgreSQL dialect
    // - postgresql-14 extends postgresql-12
    let dialects = vec![
        "base",
        "mysql-5.7",
        "mysql-8.0",
        "postgresql-12",
        "postgresql-14",
    ];

    for dialect in dialects {
        println!("cargo:warning=Building grammar for dialect: {}", dialect);

        // Set DIALECT environment variable for tree-sitter
        // Safe in build scripts: single-threaded, controlled execution
        unsafe {
            env::set_var("DIALECT", dialect);
        }

        // Run tree-sitter generate
        let status = Command::new("tree-sitter")
            .arg("generate")
            .arg("-o")
            .arg("gen")
            .current_dir(grammar_dir)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=Successfully generated {} grammar", dialect);
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
                println!("cargo:warning=Install tree-sitter-cli: npm install -g tree-sitter-cli");
                continue;
            }
        }

        // Rename generated parser.c to dialect-specific file
        let parser_c = grammar_dir.join("gen/parser.c");
        let dialect_parser_c = grammar_dir.join(format!("gen/parser-{}.c", dialect));

        if parser_c.exists() {
            // Rename to dialect-specific file in gen/
            fs::rename(&parser_c, &dialect_parser_c).expect("Failed to rename parser.c");
            println!(
                "cargo:warning=Saved {} parser as gen/parser-{}.c",
                dialect, dialect
            );

            // Also copy to out_dir for compilation
            let dest_path = Path::new(&out_dir).join(format!("parser-{}.c", dialect));
            fs::copy(&dialect_parser_c, &dest_path).expect("Failed to copy parser.c");

            // Compile the parser
            cc::Build::new()
                .file(&dest_path)
                .include(grammar_dir.join("gen"))
                .compile(&format!("parser-{}", dialect));

            println!("cargo:warning=Compiled {} parser", dialect);
        } else {
            println!("cargo:warning=parser.c not found for dialect {}", dialect);
        }
    }

    // Rebuild if grammar files change
    println!("cargo:rerun-if-changed=src/grammar/grammar.js");
    println!("cargo:rerun-if-changed=src/grammar/dialect/");
}
