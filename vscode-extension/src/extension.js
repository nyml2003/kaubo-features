const vscode = require("vscode");
const path = require("path");

let kauboDiagnostics;

async function loadWasm() {
  try {
    const wasmPath = path.join(__dirname, "..", "wasm", "kaubo_wasm.js");
    const mod = require(wasmPath);
    return mod;
  } catch (e) {
    console.warn("[kaubo] WASM not available for diagnostics:", e.message);
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

    context.subscriptions.push(
      vscode.workspace.onDidOpenTextDocument(updateDiagnostics)
    );
    context.subscriptions.push(
      vscode.workspace.onDidChangeTextDocument((e) =>
        updateDiagnostics(e.document)
      )
    );
    context.subscriptions.push(
      vscode.workspace.onDidSaveTextDocument(updateDiagnostics)
    );

    if (vscode.window.activeTextEditor) {
      updateDiagnostics(vscode.window.activeTextEditor.document);
    }
  }
}

function deactivate() {
  if (kauboDiagnostics) {
    kauboDiagnostics.dispose();
  }
}

module.exports = { activate, deactivate };
