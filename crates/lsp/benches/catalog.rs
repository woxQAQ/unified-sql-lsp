//! Catalog query performance benchmarks
//!
//! Measures the performance of catalog operations.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use unified_sql_lsp_catalog::{ColumnMetadata, DataType, TableMetadata, TableType};

fn create_test_table() -> TableMetadata {
    TableMetadata {
        schema: "test".to_string(),
        name: "users".to_string(),
        table_type: TableType::Table,
        columns: vec![
            ColumnMetadata {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                comment: None,
                is_primary_key: true,
                is_foreign_key: false,
                references: None,
            },
            ColumnMetadata {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: true,
                default_value: None,
                comment: None,
                is_primary_key: false,
                is_foreign_key: false,
                references: None,
            },
        ],
        row_count_estimate: None,
        comment: None,
    }
}

fn bench_table_metadata_creation(c: &mut Criterion) {
    c.bench_function("catalog/table_metadata_creation", |b| {
        b.iter(|| {
            let table = create_test_table();
            black_box(table);
        });
    });
}

fn bench_column_access(c: &mut Criterion) {
    let table = create_test_table();

    c.bench_function("catalog/column_access", |b| {
        b.iter(|| {
            let _cols = &table.columns;
            black_box(&_cols);
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = bench_table_metadata_creation, bench_column_access
);

criterion_main!(benches);
