import {
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { complete as wasmComplete } from "@kaubo/wasm";

interface ServiceCompletion {
  label: string;
  kind: string;
  detail?: string | null;
}

function completionsFromLanguageService(
  source: string,
  offset: number,
): Completion[] {
  try {
    const parsed = JSON.parse(
      wasmComplete(source, offset),
    ) as ServiceCompletion[];
    return parsed.map((item) => {
      const isMethod = item.kind === "method";
      const completion: Completion = {
        label: isMethod ? `${item.label}()` : item.label,
        type: item.kind === "field" ? "property" : item.kind,
        boost: 3,
      };
      if (item.detail) {
        completion.detail = item.detail;
      }
      if (isMethod) {
        completion.apply = `${item.label}()`;
        // Place cursor between the parens
        completion.section = "method";
      }
      return completion;
    });
  } catch {
    return [];
  }
}

/** Find the last `.` before the cursor — handles `obj.`, `obj.f`, `1.to_float().` */
function dotPrefix(
  source: string,
  pos: number,
): { from: number } | null {
  const text = source.slice(0, pos);
  const dotIdx = text.lastIndexOf(".");
  if (dotIdx < 0) return null;
  // Skip if immediately after a digit (float literal like `1.0`)
  if (dotIdx + 1 < pos && /\d/.test(source[dotIdx + 1])) {
    // Might still be valid for `1.to_float` — check if there's an identifier-like pattern
    // after the dot that's not purely numeric
    const after = source.slice(dotIdx + 1, pos);
    if (/^\d+$/.test(after)) return null; // float literal: skip
  }
  return { from: dotIdx + 1 };
}

export function kauboCompletions(
  context: CompletionContext,
): CompletionResult | null {
  const source = context.state.doc.toString();

  // Dot-access: any text after the last `.` (handles `1.|`, `1.t|`, `1.to_float().|`)
  const dotInfo = dotPrefix(source, context.pos);
  if (dotInfo) {
    const items = completionsFromLanguageService(source, context.pos);
    if (items.length > 0) {
      return { from: dotInfo.from, options: items };
    }
  }

  // Regular word completions via WASM
  const word = context.matchBefore(/\w*/);
  if (!word || (word.from === word.to && !context.explicit)) {
    return null;
  }

  const items = completionsFromLanguageService(source, context.pos);
  if (items.length === 0) return null;

  return { from: word.from, options: items };
}
