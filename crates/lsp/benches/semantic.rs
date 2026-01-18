//! Semantic analysis performance benchmarks
//!
//! Measures the performance of semantic analysis components:
//! - Scope creation and management
//! - Symbol resolution

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use unified_sql_lsp_catalog::DataType;
use unified_sql_lsp_semantic::{ColumnSymbol, ScopeManager, ScopeType, TableSymbol};

fn bench_scope_creation(c: &mut Criterion) {
    c.bench_function("semantic/scope_creation", |b| {
        b.iter(|| {
            let mut manager = ScopeManager::new();
            let scope_id = manager.create_scope(ScopeType::Query, None);
            black_box(scope_id);
        });
    });
}

fn bench_scope_with_tables(c: &mut Criterion) {
    c.bench_function("semantic/scope_with_tables", |b| {
        b.iter(|| {
            let mut manager = ScopeManager::new();
            let scope_id = manager.create_scope(ScopeType::Query, None);

            let table = TableSymbol::new("users").with_columns(vec![
                ColumnSymbol::new("id", DataType::Integer, "users"),
                ColumnSymbol::new("name", DataType::Text, "users"),
                ColumnSymbol::new("email", DataType::Text, "users"),
            ]);

            let scope = manager.get_scope_mut(scope_id).unwrap();
            scope.add_table(table).unwrap();
            black_box(scope_id);
        });
    });
}

fn bench_table_resolution(c: &mut Criterion) {
    let mut manager = ScopeManager::new();
    let scope_id = manager.create_scope(ScopeType::Query, None);

    let table = TableSymbol::new("users").with_columns(vec![
        ColumnSymbol::new("id", DataType::Integer, "users"),
        ColumnSymbol::new("name", DataType::Text, "users"),
    ]);

    let scope = manager.get_scope_mut(scope_id).unwrap();
    scope.add_table(table).unwrap();

    c.bench_function("semantic/table_resolution", |b| {
        b.iter(|| {
            let result = manager.resolve_table("users", scope_id);
            black_box(result);
        });
    });
}

fn bench_nested_scopes(c: &mut Criterion) {
    c.bench_function("semantic/nested_scopes", |b| {
        b.iter(|| {
            let mut manager = ScopeManager::new();

            // Create parent scope
            let parent_id = manager.create_scope(ScopeType::Query, None);
            let table = TableSymbol::new("users");
            manager
                .get_scope_mut(parent_id)
                .unwrap()
                .add_table(table)
                .unwrap();

            // Create child scope
            let child_id = manager.create_scope(ScopeType::Subquery, Some(parent_id));

            // Resolve from child
            let result = manager.resolve_table("users", child_id);
            black_box(result);
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    targets =
        bench_scope_creation,
        bench_scope_with_tables,
        bench_table_resolution,
        bench_nested_scopes
);

criterion_main!(benches);
