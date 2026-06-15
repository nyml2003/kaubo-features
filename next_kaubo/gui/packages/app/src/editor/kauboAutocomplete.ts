import { type Completion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";

const KEYWORDS = [
  "var", "if", "else", "elif", "while", "for", "return",
  "in", "yield", "break", "continue", "pass",
  "struct", "impl", "import", "as", "from",
  "pub", "json", "module", "operator",
  "and", "or", "not",
];

const ATOMS = ["true", "false", "null"];

const BUILTINS = [
  "print", "assert", "type", "to_string",
  "sqrt", "sin", "cos", "floor", "ceil",
  "len", "push", "is_empty", "range", "clone",
  "read_file", "write_file", "exists", "is_file", "is_dir",
  "substring", "contains", "starts_with", "ends_with",
  "length", "trim", "split", "join", "replace",
  "to_lower", "to_upper",
  "now_timestamp", "format_time",
  "sha256", "base64_encode", "base64_decode",
  "random", "random_int",
  "create_coroutine", "resume", "coroutine_status",
];

const CONSTANTS = [
  { label: "PI", detail: "≈ 3.14159" },
  { label: "E", detail: "≈ 2.71828" },
];

export function kauboCompletions(context: CompletionContext): CompletionResult | null {
  const word = context.matchBefore(/\w*/);
  if (!word || (word.from === word.to && !context.explicit)) {
    return null;
  }

  const prefix = word.text.toLowerCase();
  const options: Completion[] = [];

  for (const kw of KEYWORDS) {
    if (kw.startsWith(prefix)) {
      options.push({
        label: kw,
        type: "keyword",
        boost: 2,
      });
    }
  }

  for (const atom of ATOMS) {
    if (atom.startsWith(prefix)) {
      options.push({
        label: atom,
        type: "constant",
        boost: 2,
      });
    }
  }

  for (const name of BUILTINS) {
    if (name.startsWith(prefix)) {
      options.push({
        label: name,
        type: "function",
        detail: "builtin",
        boost: 1,
      });
    }
  }

  for (const c of CONSTANTS) {
    if (c.label.toLowerCase().startsWith(prefix)) {
      options.push({
        label: c.label,
        type: "constant",
        detail: c.detail,
        boost: 1,
      });
    }
  }

  if (options.length === 0) return null;
  return { from: word.from, options };
}
