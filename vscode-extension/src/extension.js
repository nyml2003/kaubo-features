const vscode = require("vscode");
const path = require("path");

let kauboDiagnostics;

async function loadWasm() {
  try {
    const wasmPath = path.join(__dirname, "..", "wasm", "kaubo_wasm.js");
    const mod = require(wasmPath);
    return mod;
  } catch (e) {
    console.warn("[kaubo] WASM not available:", e.message);
    return null;
  }
}

async function activate(context) {
  const wasm = await loadWasm();

  kauboDiagnostics = vscode.languages.createDiagnosticCollection("kaubo");

  if (wasm && wasm.diagnose) {
    context.subscriptions.push(kauboDiagnostics);

    function updateDiagnostics(document) {
      if (document.languageId !== "kaubo") return;

      try {
        const source = document.getText();
        const json = wasm.diagnose(source);
        const errors = JSON.parse(json);

        const diagnostics = [];

        for (const err of errors) {
          const line = (err.line || 1) - 1;
          const col = (err.column || 1) - 1;

          let range;
          if (err.from !== undefined && err.to !== undefined) {
            const startPos = document.positionAt(err.from);
            const endPos = document.positionAt(err.to);
            range = new vscode.Range(startPos, endPos);
          } else {
            const pos = new vscode.Position(line, col);
            range = new vscode.Range(pos, pos);
          }

          const severity =
            err.severity === "warning"
              ? vscode.DiagnosticSeverity.Warning
              : vscode.DiagnosticSeverity.Error;

          const diagnostic = new vscode.Diagnostic(
            range,
            err.message,
            severity
          );
          diagnostic.source = "kaubo";
          diagnostics.push(diagnostic);
        }

        kauboDiagnostics.set(document.uri, diagnostics);
      } catch (e) {
        kauboDiagnostics.set(document.uri, []);
        console.warn("[kaubo] Diagnose error:", e.message);
      }
    }

    // ── LSP: feed source changes to coordinator ──

    function updateLsp(document) {
      if (document.languageId !== "kaubo") return;
      try {
        if (wasm.lsp_on_change) {
          wasm.lsp_on_change(document.getText());
        }
      } catch (e) {
        // LSP coordinator may not be available in all WASM builds
      }
    }

    context.subscriptions.push(
      vscode.workspace.onDidOpenTextDocument((doc) => {
        updateDiagnostics(doc);
        updateLsp(doc);
      })
    );
    context.subscriptions.push(
      vscode.workspace.onDidChangeTextDocument((e) => {
        updateDiagnostics(e.document);
        updateLsp(e.document);
      })
    );
    context.subscriptions.push(
      vscode.workspace.onDidSaveTextDocument((doc) => {
        updateDiagnostics(doc);
        updateLsp(doc);
      })
    );

    if (vscode.window.activeTextEditor) {
      updateDiagnostics(vscode.window.activeTextEditor.document);
      updateLsp(vscode.window.activeTextEditor.document);
    }

    // ── Hover Provider ──

    if (wasm.hover) {
      context.subscriptions.push(
        vscode.languages.registerHoverProvider("kaubo", {
          provideHover(document, position) {
            try {
              const offset = document.offsetAt(position);
              const json = wasm.hover(document.getText(), offset);
              if (json === "null") return null;
              const info = JSON.parse(json);
              const contents = [];
              if (info.type) {
                contents.push(`**${info.kind}**: \`${info.type}\``);
              } else {
                contents.push(`**${info.kind}**`);
              }
              if (info.description) {
                contents.push(info.description);
              }
              return new vscode.Hover(contents.join("\n\n"));
            } catch (e) {
              return null;
            }
          },
        })
      );
    }

    // ── Definition Provider (go-to-definition) ──

    if (wasm.goto_def) {
      context.subscriptions.push(
        vscode.languages.registerDefinitionProvider("kaubo", {
          provideDefinition(document, position) {
            try {
              const offset = document.offsetAt(position);
              const json = wasm.goto_def(document.getText(), offset);
              if (json === "null") return null;
              const target = JSON.parse(json);
              const pos = new vscode.Position(
                Math.max(0, target.line - 1),
                Math.max(0, target.col - 1)
              );
              return new vscode.Location(document.uri, pos);
            } catch (e) {
              return null;
            }
          },
        })
      );
    }

    // ── Completion Provider ──

    if (wasm.complete) {
      context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
          "kaubo",
          {
            provideCompletionItems(document, position) {
              try {
                const offset = document.offsetAt(position);
                const json = wasm.complete(document.getText(), offset);
                const items = JSON.parse(json);
                return items.map((item) => {
                  const kind = mapCompletionKind(item.kind);
                  const completion = new vscode.CompletionItem(
                    item.label,
                    kind
                  );
                  if (item.detail) {
                    completion.detail = item.detail;
                  }
                  return completion;
                });
              } catch (e) {
                return [];
              }
            },
          },
          "." // trigger on dot for member access
        )
      );
    }

    // ── Semantic Tokens Provider ──

    if (wasm.semantic_tokens) {
      const tokenTypes = [
        "keyword",
        "number",
        "string",
        "comment",
        "operator",
        "type",
        "function",
        "method",
        "field",
        "identifier",
        "atom",
      ];
      const tokenModifiers = [];
      const legend = new vscode.SemanticTokensLegend(
        tokenTypes,
        tokenModifiers
      );

      context.subscriptions.push(
        vscode.languages.registerDocumentSemanticTokensProvider(
          "kaubo",
          {
            provideDocumentSemanticTokens(document) {
              try {
                const json = wasm.semantic_tokens(document.getText());
                const tokens = JSON.parse(json);
                const builder = new vscode.SemanticTokensBuilder(legend);

                for (const token of tokens) {
                  const startPos = document.positionAt(token.from);
                  const endPos = document.positionAt(token.to);
                  const typeIdx = tokenTypes.indexOf(token.kind);
                  if (typeIdx >= 0) {
                    const line = startPos.line;
                    const startChar = startPos.character;
                    const length = endPos.character - startChar;
                    builder.push(line, startChar, length, typeIdx, 0);
                  }
                }

                return builder.build();
              } catch (e) {
                return new vscode.SemanticTokensBuilder(legend).build();
              }
            },
          },
          legend
        )
      );
        // ── Inlay Hints Provider ──

        if (wasm.inlay_hints) {
          context.subscriptions.push(
            vscode.languages.registerInlayHintsProvider("kaubo", {
              provideInlayHints(document, range, token) {
                try {
                  const json = wasm.inlay_hints(document.getText());
                  const rawHints = JSON.parse(json);
                  return rawHints.map((hint) => {
                    const pos = document.positionAt(hint.position);
                    return new vscode.InlayHint(
                      pos,
                      hint.label,
                      vscode.InlayHintKind.Parameter
                    );
                  });
                } catch (e) {
                  return [];
                }
              },
            })
          );
        }
    }
  }
}

function mapCompletionKind(kind) {
  switch (kind) {
    case "function":
      return vscode.CompletionItemKind.Function;
    case "method":
      return vscode.CompletionItemKind.Method;
    case "field":
      return vscode.CompletionItemKind.Field;
    case "variable":
    case "var":
    case "const":
      return vscode.CompletionItemKind.Variable;
    case "struct":
    case "enum":
    case "interface":
      return vscode.CompletionItemKind.Class;
    default:
      return vscode.CompletionItemKind.Text;
  }
}

function deactivate() {
  if (kauboDiagnostics) {
    kauboDiagnostics.dispose();
  }
}

module.exports = { activate, deactivate };
