/**
 * Monaco Editor integration with LSP client
 *
 * This module sets up Monaco editor with custom completion, hover, and
 * diagnostic providers backed by the LSP WebSocket client.
 */

import * as monaco from 'monaco-editor';
import { LspClient, CompletionItem, Diagnostic } from './lsp-client';

/**
 * Set up Monaco editor with LSP features
 */
export function setupMonacoWithLSP(
  container: HTMLDivElement,
  lspClient: LspClient,
  options: monaco.editor.IStandaloneEditorConstructionOptions = {}
): monaco.editor.IStandaloneCodeEditor {
  // Register SQL language if not already registered
  if (!monaco.languages.getLanguages().some((lang) => lang.id === 'sql')) {
    monaco.languages.register({ id: 'sql' });
  }

  // Create editor
  const editor = monaco.editor.create(container, {
    language: 'sql',
    theme: 'vs-dark',
    automaticLayout: true,
    minimap: { enabled: false },
    fontSize: 14,
    lineNumbers: 'on',
    scrollBeyondLastLine: false,
    suggest: {
      showIcons: true,
      showSnippets: true,
    },
    ...options,
  });

  // Get the model and notify LSP server when document opens
  const model = editor.getModel();
  if (model) {
    // Send didOpen notification when document is first created
    const documentUri = 'file:///playground.sql';
    const initialContent = model.getValue();
    const languageId = model.getLanguageId();

    // Send didOpen notification asynchronously to ensure connection is ready
    setTimeout(async () => {
      try {
        await lspClient.sendNotification('textDocument/didOpen', {
          textDocument: {
            uri: documentUri,
            languageId: languageId,
            version: 1,
            text: initialContent,
          },
        });
        console.log('[Monaco] Sent didOpen notification');
      } catch (error) {
        console.error('[Monaco] Failed to send didOpen:', error);
      }
    }, 100);

    // Track content changes for didChange notifications
    let version = 1;
    model.onDidChangeContent((event) => {
      version++;
      const changes = event.changes.map((change) => ({
        range: {
          start: { line: change.range.startLineNumber - 1, character: change.range.startColumn - 1 },
          end: { line: change.range.endLineNumber - 1, character: change.range.endColumn - 1 },
        },
        rangeLength: change.rangeLength,
        text: change.text,
      }));

      lspClient.sendNotification('textDocument/didChange', {
        textDocument: {
          uri: documentUri,
          version: version,
        },
        contentChanges: changes,
      }).catch((error) => {
        console.error('[Monaco] Failed to send didChange:', error);
      });
    });
  }

  // Register completion provider
  const completionProvider = monaco.languages.registerCompletionItemProvider('sql', {
    triggerCharacters: ['.', ' '],

    async provideCompletionItems(model, position, _context, _token) {
      if (!lspClient.isConnected()) {
        return { suggestions: [] };
      }

      const text = model.getValue();
      const lineNumber = position.lineNumber - 1; // Monaco is 1-based, LSP is 0-based
      const column = position.column - 1;

      try {
        const items = await lspClient.completion(text, lineNumber, column);

        return {
          suggestions: items.map((item: CompletionItem) => {
            const suggestion: monaco.languages.CompletionItem = {
              label: item.label,
              kind: convertCompletionKind(item.kind),
              detail: item.detail,
              documentation: item.documentation
                ? { value: item.documentation, isTrusted: true }
                : undefined,
              insertText: item.insertText || item.label,
              sortText: item.sortText,
              filterText: item.filterText,
              range: undefined as any, // Using as any for compatibility
            };
            return suggestion;
          }),
        };
      } catch (error) {
        console.error('[Monaco] Completion error:', error);
        return { suggestions: [] };
      }
    },
  });

  // Register hover provider
  const hoverProvider = monaco.languages.registerHoverProvider('sql', {
    async provideHover(model, position) {
      if (!lspClient.isConnected()) {
        return null;
      }

      const text = model.getValue();
      const lineNumber = position.lineNumber - 1;
      const column = position.column - 1;

      try {
        const hover = await lspClient.hover(text, lineNumber, column);

        if (!hover) {
          return null;
        }

        return {
          range: hover.range
            ? new monaco.Range(
                hover.range.start.line + 1,
                hover.range.start.character + 1,
                hover.range.end.line + 1,
                hover.range.end.character + 1
              )
            : new monaco.Range(
                position.lineNumber,
                position.column,
                position.lineNumber,
                position.column
              ),
          contents: [{ value: hover.contents.value, isTrusted: true }],
        };
      } catch (error) {
        console.error('[Monaco] Hover error:', error);
        return null;
      }
    },
  });

  // Store providers for cleanup
  (editor as any)._lspProviders = {
    completion: completionProvider,
    hover: hoverProvider,
  };

  return editor;
}

/**
 * Update diagnostics in Monaco editor
 */
export function updateDiagnostics(
  editor: monaco.editor.IStandaloneCodeEditor,
  diagnostics: Diagnostic[]
) {
  const model = editor.getModel();
  if (!model) {
    return;
  }

  const monacoMarkers = diagnostics.map((diag) => ({
    severity: convertSeverity(diag.severity),
    message: diag.message,
    startLineNumber: diag.range.start.line + 1,
    startColumn: diag.range.start.character + 1,
    endLineNumber: diag.range.end.line + 1,
    endColumn: diag.range.end.character + 1,
    source: diag.source || 'LSP',
    code: typeof diag.code === 'string' ? diag.code : String(diag.code || ''),
  }));

  monaco.editor.setModelMarkers(model, 'lsp', monacoMarkers);
}

/**
 * Clear all diagnostics
 */
export function clearDiagnostics(editor: monaco.editor.IStandaloneCodeEditor) {
  const model = editor.getModel();
  if (model) {
    monaco.editor.setModelMarkers(model, 'lsp', []);
  }
}

/**
 * Dispose LSP providers
 */
export function disposeLSPProviders(editor: monaco.editor.IStandaloneCodeEditor) {
  const providers = (editor as any)._lspProviders;
  if (providers) {
    providers.completion?.dispose();
    providers.hover?.dispose();
    delete (editor as any)._lspProviders;
  }
}

/**
 * Convert LSP completion kind to Monaco completion kind
 */
function convertCompletionKind(kind?: number): monaco.languages.CompletionItemKind {
  if (!kind) {
    return monaco.languages.CompletionItemKind.Text;
  }

  // LSP to Monaco kind mapping
  const kindMap: { [key: number]: monaco.languages.CompletionItemKind } = {
    1: monaco.languages.CompletionItemKind.Text,
    2: monaco.languages.CompletionItemKind.Method,
    3: monaco.languages.CompletionItemKind.Function,
    4: monaco.languages.CompletionItemKind.Constructor,
    5: monaco.languages.CompletionItemKind.Field,
    6: monaco.languages.CompletionItemKind.Variable,
    7: monaco.languages.CompletionItemKind.Class,
    8: monaco.languages.CompletionItemKind.Interface,
    9: monaco.languages.CompletionItemKind.Module,
    10: monaco.languages.CompletionItemKind.Property,
    11: monaco.languages.CompletionItemKind.Unit,
    12: monaco.languages.CompletionItemKind.Value,
    13: monaco.languages.CompletionItemKind.Enum,
    14: monaco.languages.CompletionItemKind.Keyword,
    15: monaco.languages.CompletionItemKind.Snippet,
    16: monaco.languages.CompletionItemKind.Color,
    17: monaco.languages.CompletionItemKind.File,
    18: monaco.languages.CompletionItemKind.Reference,
    19: monaco.languages.CompletionItemKind.Folder,
    20: monaco.languages.CompletionItemKind.EnumMember,
    21: monaco.languages.CompletionItemKind.Constant,
    22: monaco.languages.CompletionItemKind.Struct,
    23: monaco.languages.CompletionItemKind.Event,
    24: monaco.languages.CompletionItemKind.Operator,
    25: monaco.languages.CompletionItemKind.TypeParameter,
  };

  return kindMap[kind] || monaco.languages.CompletionItemKind.Text;
}

/**
 * Convert LSP severity to Monaco severity
 */
function convertSeverity(severity: number): monaco.MarkerSeverity {
  switch (severity) {
    case 1: // Error
      return monaco.MarkerSeverity.Error;
    case 2: // Warning
      return monaco.MarkerSeverity.Warning;
    case 3: // Info
      return monaco.MarkerSeverity.Info;
    case 4: // Hint
      return monaco.MarkerSeverity.Hint;
    default:
      return monaco.MarkerSeverity.Info;
  }
}
