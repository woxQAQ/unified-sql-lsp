package protocol

// Core LSP types for the unified SQL LSP server
// This package defines shared types used across internal packages

// Position represents a position in a text document
type Position struct {
	Line      int `json:"line"`
	Character int `json:"character"`
}

// Range represents a range in a text document
type Range struct {
	Start Position `json:"start"`
	End   Position `json:"end"`
}

// TextEdit represents a text edit
type TextEdit struct {
	Range   Range  `json:"range"`
	NewText string `json:"newText"`
}

// CompletionItemKind represents the kind of completion item
type CompletionItemKind int

const (
	CompletionItemKindKeyword CompletionItemKind = iota + 1
	CompletionItemKindFunction
	CompletionItemKindTable
	CompletionItemKindColumn
	CompletionItemKindSchema
	CompletionItemKindView
)

// CompletionItem represents a completion item
type CompletionItem struct {
	Label         string             `json:"label"`
	Kind          CompletionItemKind `json:"kind"`
	Detail        string             `json:"detail,omitempty"`
	Documentation string             `json:"documentation,omitempty"`
	TextEdit      *TextEdit          `json:"textEdit,omitempty"`
	SortText      string             `json:"sortText,omitempty"`
	FilterText    string             `json:"filterText,omitempty"`
}
