use crate::fixtures::{TestQuery, load_test_queries};
use crate::operations::{execute_completion, execute_hover, execute_diagnostics, Document};
use unified_sql_lsp_context::Position;
use lsp_types::Url;

pub struct WorkloadResult {
    pub operations_executed: usize,
    pub total_duration_ns: u128,
}

/// Run completion scenario on a single query
pub fn run_completion_scenario(query: &TestQuery) -> WorkloadResult {
    let doc = create_document(query);
    let position = find_completion_position(&doc);

    let result = execute_completion(&doc, position).unwrap();

    WorkloadResult {
        operations_executed: 1,
        total_duration_ns: result.duration_ns,
    }
}

/// Run hover scenario on a single query
pub fn run_hover_scenario(query: &TestQuery) -> WorkloadResult {
    let doc = create_document(query);
    let position = find_hover_position(&doc);

    let result = execute_hover(&doc, position).unwrap();

    WorkloadResult {
        operations_executed: 1,
        total_duration_ns: result.duration_ns,
    }
}

/// Run diagnostics scenario on a single query
pub fn run_diagnostics_scenario(query: &TestQuery) -> WorkloadResult {
    let doc = create_document(query);

    let result = execute_diagnostics(&doc).unwrap();

    WorkloadResult {
        operations_executed: 1,
        total_duration_ns: result.duration_ns,
    }
}

/// Simulate realistic editing session with mixed operations
pub fn simulate_editing_session() -> WorkloadResult {
    let queries = load_test_queries();
    let mut total_operations = 0;
    let mut total_duration = 0;

    for (_, query) in queries.iter().take(3) {
        // Simulate document open
        let mut doc = create_document(query);

        // Initial diagnostics
        let diag_result = execute_diagnostics(&doc).unwrap();
        total_duration += diag_result.duration_ns;
        total_operations += 1;

        // Simulate some edits
        simulate_edits(&mut doc);

        // Re-run diagnostics after edit
        let diag_result = execute_diagnostics(&doc).unwrap();
        total_duration += diag_result.duration_ns;
        total_operations += 1;

        // Try completion at various positions
        for _ in 0..3 {
            let pos = find_completion_position(&doc);
            let comp_result = execute_completion(&doc, pos).unwrap();
            total_duration += comp_result.duration_ns;
            total_operations += 1;
        }
    }

    WorkloadResult {
        operations_executed: total_operations,
        total_duration_ns: total_duration,
    }
}

fn create_document(query: &TestQuery) -> Document {
    // Create a document with the query content
    let uri = Url::parse("file:///test.sql").unwrap();
    Document::new(query.sql.clone(), uri)
}

fn find_completion_position(_doc: &Document) -> Position {
    // Find SELECT clause position (simplified: line 0, char 10)
    Position { line: 0, character: 10 }
}

fn find_hover_position(_doc: &Document) -> Position {
    // Find a column reference position (simplified)
    Position { line: 0, character: 10 }
}

fn simulate_edits(_doc: &mut Document) {
    // Apply 1-3 character changes to simulate typing
    // For now, this is a placeholder - actual edits would require
    // implementing the apply_document_change function from operations module
}
