//! Tree-sitter parsing performance benchmarks
//!
//! Measures the performance of Tree-sitter parsing across:
//! - Query complexity levels (simple, medium, complex)
//! - Dialects (MySQL, PostgreSQL)

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use unified_sql_grammar::language_for_dialect;
use unified_sql_lsp_ir::Dialect;

/// Load a fixture SQL file
fn load_fixture(dialect: Dialect, complexity: &str, index: usize) -> String {
    let dialect_name = match dialect {
        Dialect::MySQL => "mysql",
        Dialect::PostgreSQL => "postgresql",
        _ => panic!("Unsupported dialect for benchmark: {:?}", dialect),
    };

    // Map index to filename suffix
    let suffix = match index {
        1 => "01_single_table",
        2 => "02_basic_where",
        3 => "03_order_by",
        _ => panic!("Invalid fixture index: {}", index),
    };

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = format!(
        "{}/benches/fixtures/{}{}_{}.sql",
        manifest_dir, complexity, dialect_name, suffix
    );

    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", path, e))
}

/// Benchmark parsing a single query
fn bench_parse_query(c: &mut Criterion, dialect: Dialect, complexity: &str, index: usize) {
    let query = load_fixture(dialect, complexity, index);
    let language = language_for_dialect(dialect)
        .unwrap_or_else(|| panic!("No language for dialect: {:?}", dialect));

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(language)
        .expect("Failed to set language");

    let mut group = c.benchmark_group(format!("parsing/{:?}/{}", dialect, complexity));

    group.throughput(Throughput::Bytes(query.len() as u64));

    group.bench_function(BenchmarkId::from_parameter(index), |b| {
        b.iter(|| {
            let tree = parser.parse(black_box(&query), None);
            assert!(tree.is_some(), "Parsing failed");
            black_box(tree);
        });
    });

    group.finish();
}

fn bench_parsing_simple(c: &mut Criterion) {
    for dialect in [Dialect::MySQL, Dialect::PostgreSQL] {
        for index in 1..=3 {
            bench_parse_query(c, dialect, "simple", index);
        }
    }
}

fn bench_parsing_dialect_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing/dialect_comparison");

    for (complexity, index) in [("simple", 1)] {
        for dialect in [Dialect::MySQL, Dialect::PostgreSQL] {
            let query = load_fixture(dialect, complexity, index);
            let language = language_for_dialect(dialect).unwrap();

            group.bench_function(
                BenchmarkId::new(
                    format!("{:?}", dialect),
                    format!("{}_{}", complexity, index),
                ),
                |b| {
                    let mut parser = tree_sitter::Parser::new();
                    parser.set_language(language).unwrap();

                    b.iter(|| {
                        let tree = parser.parse(black_box(&query), None);
                        black_box(tree);
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets =
        bench_parsing_simple,
        bench_parsing_dialect_comparison
);

criterion_main!(benches);
