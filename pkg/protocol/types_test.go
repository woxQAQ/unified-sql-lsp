package protocol

import "testing"

func TestCompletionItemKind(t *testing.T) {
	kinds := []CompletionItemKind{
		CompletionItemKindKeyword,
		CompletionItemKindFunction,
		CompletionItemKindTable,
		CompletionItemKindColumn,
		CompletionItemKindSchema,
		CompletionItemKindView,
		CompletionItemKindSequence,
		CompletionItemKindEnum,
		CompletionItemKindType,
		CompletionItemKindOperator,
		CompletionItemKindParameter,
		CompletionItemKindSnippet,
		CompletionItemKindReference,
		CompletionItemKindNamespace,
		CompletionItemKindStruct,
		CompletionItemKindModule,
	}

	for i, kind := range kinds {
		if kind != CompletionItemKind(i+1) {
			t.Errorf("Kind mismatch: got %d, want %d", kind, i+1)
		}
	}
}

func TestPosition(t *testing.T) {
	pos := Position{Line: 1, Character: 5}
	if pos.Line != 1 {
		t.Errorf("Line mismatch: got %d, want %d", pos.Line, 1)
	}
	if pos.Character != 5 {
		t.Errorf("Character mismatch: got %d, want %d", pos.Character, 5)
	}
}

func TestRange(t *testing.T) {
	r := Range{
		Start: Position{Line: 0, Character: 0},
		End:   Position{Line: 1, Character: 5},
	}

	if r.Start.Line != 0 {
		t.Errorf("Start line mismatch: got %d, want %d", r.Start.Line, 0)
	}
	if r.End.Character != 5 {
		t.Errorf("End character mismatch: got %d, want %d", r.End.Character, 5)
	}
}
