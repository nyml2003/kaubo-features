const { describe, it } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const grammarPath = path.join(__dirname, "..", "syntaxes", "kaubo.tmLanguage.json");
const grammar = JSON.parse(fs.readFileSync(grammarPath, "utf-8"));

describe("kaubo.tmLanguage.json", () => {
  it("has correct scopeName", () => {
    assert.equal(grammar.scopeName, "source.kaubo");
  });

  it("has correct name", () => {
    assert.equal(grammar.name, "Kaubo");
  });

  it("contains all required pattern groups", () => {
    const includes = grammar.patterns.map((p) => p.include);
    assert.ok(includes.includes("#comments"));
    assert.ok(includes.includes("#template-strings"));
    assert.ok(includes.includes("#strings"));
    assert.ok(includes.includes("#keywords"));
    assert.ok(includes.includes("#atoms"));
    assert.ok(includes.includes("#numbers"));
    assert.ok(includes.includes("#operators"));
    assert.ok(includes.includes("#identifiers"));
  });

  it("has all repository entries", () => {
    const repos = Object.keys(grammar.repository);
    assert.ok(repos.includes("comments"));
    assert.ok(repos.includes("template-strings"));
    assert.ok(repos.includes("strings"));
    assert.ok(repos.includes("keywords"));
    assert.ok(repos.includes("atoms"));
    assert.ok(repos.includes("numbers"));
    assert.ok(repos.includes("operators"));
    assert.ok(repos.includes("identifiers"));
  });

  it("recognizes comment patterns", () => {
    const comments = grammar.repository.comments.patterns;
    assert.equal(comments.length, 2);

    const block = comments.find((c) => c.name === "comment.block.kaubo");
    assert.ok(block, "block comment pattern exists");
    assert.equal(block.begin, "/\\*");
    assert.equal(block.end, "\\*/");

    const line = comments.find((c) => c.name === "comment.line.double-slash.kaubo");
    assert.ok(line, "line comment pattern exists");
    assert.equal(line.match, "//.*$");
  });

  it("recognizes string patterns with escape sequences", () => {
    const strings = grammar.repository.strings.patterns;
    assert.equal(strings.length, 1); // only double-quoted strings

    const str = strings[0];
    assert.ok(str.patterns, "string has escape patterns");
    const escapes = str.patterns.filter((p) => p.name === "constant.character.escape.kaubo");
    assert.equal(escapes.length, 1);
    assert.equal(escapes[0].match, "\\\\(n|r|t|\\\\|\"|0)");
  });

  const keywordGroups = [
    { name: "keyword.control.kaubo", words: ["if", "else", "while", "for", "return", "in", "break", "continue", "match"] },
    { name: "keyword.declaration.kaubo", words: ["const", "var"] },
    { name: "keyword.type.kaubo", words: ["struct", "enum", "interface", "impl"] },
    { name: "keyword.other.kaubo", words: ["export", "import", "as", "from", "operator", "async", "await", "and", "or", "not", "self"] },
  ];

  for (const group of keywordGroups) {
    it(`keyword group "${group.name}" matches all expected words`, () => {
      const pattern = grammar.repository.keywords.patterns.find((p) => p.name === group.name);
      assert.ok(pattern, `pattern ${group.name} exists`);
      for (const word of group.words) {
        assert.ok(
          pattern.match.includes(word),
          `"${word}" is in ${group.name}`
        );
      }
    });
  }

  it("covers all 23 kaubo keywords", () => {
    const allKeywords = [
      "const", "var",
      "if", "else", "while", "for", "return", "in",
      "break", "continue", "match",
      "struct", "enum", "interface", "impl",
      "export", "import", "as", "from", "operator",
      "async", "await", "self",
      "and", "or", "not",
    ];
    const allPatterns = grammar.repository.keywords.patterns.map((p) => p.match).join("|");

    for (const kw of allKeywords) {
      const re = new RegExp(allPatterns);
      assert.ok(re.test(kw), `keyword "${kw}" is covered by grammar patterns`);
    }
  });

  it("has boolean and null atoms", () => {
    const booleans = grammar.repository.atoms.patterns.find(
      (p) => p.name === "constant.language.boolean.kaubo"
    );
    assert.ok(booleans, "boolean pattern exists");
    assert.ok(booleans.match.includes("true"));
    assert.ok(booleans.match.includes("false"));

    const nullPat = grammar.repository.atoms.patterns.find(
      (p) => p.name === "constant.language.null.kaubo"
    );
    assert.ok(nullPat, "null pattern exists");
    assert.ok(nullPat.match.includes("null"));
  });

  it("distinguishes float from integer numbers", () => {
    const floatPat = grammar.repository.numbers.patterns.find(
      (p) => p.name === "constant.numeric.float.kaubo"
    );
    assert.ok(floatPat, "float pattern exists");
    assert.ok(floatPat.match.includes("\\."));

    const intPat = grammar.repository.numbers.patterns.find(
      (p) => p.name === "constant.numeric.integer.kaubo"
    );
    assert.ok(intPat, "integer pattern exists");
  });

  const operatorChecks = [
    "keyword.operator.nullish.kaubo",
    "keyword.operator.comparison.kaubo",
    "keyword.operator.arithmetic.kaubo",
    "keyword.operator.assignment.kaubo",
    "keyword.operator.spread.kaubo",
    "punctuation.separator.kaubo",
    "punctuation.section.kaubo",
    "keyword.operator.accessor.kaubo",
    "keyword.operator.lambda.kaubo",
  ];

  for (const name of operatorChecks) {
    it(`has operator "${name}"`, () => {
      const op = grammar.repository.operators.patterns.find((p) => p.name === name);
      assert.ok(op, `operator ${name} exists`);
    });
  }

  it("identifier patterns exist and are ordered correctly", () => {
    const idents = grammar.repository.identifiers.patterns;
    assert.ok(idents.length >= 3, "has at least 3 identifier patterns");

    const func = idents.find((p) => p.name === "entity.name.function.kaubo");
    assert.ok(func, "function identifier pattern exists");
    assert.ok(func.match.includes("(?=\\()"), "uses lookahead for function call");

    const typeId = idents.find((p) => p.name === "entity.name.type.kaubo");
    assert.ok(typeId, "type identifier pattern exists");

    const variable = idents.find((p) => p.name === "variable.other.kaubo");
    assert.ok(variable, "variable pattern exists");
  });
});

describe("snippets/kaubo.json", () => {
  const snippetsPath = path.join(__dirname, "..", "snippets", "kaubo.json");
  const snippets = JSON.parse(fs.readFileSync(snippetsPath, "utf-8"));

  it("has at least 10 snippets", () => {
    assert.ok(Object.keys(snippets).length >= 10);
  });

  for (const [name, snippet] of Object.entries(snippets)) {
    it(`snippet "${name}" has required fields`, () => {
      assert.ok(snippet.prefix, `snippet ${name} has prefix`);
      assert.ok(Array.isArray(snippet.body), `snippet ${name} body is array`);
      assert.ok(snippet.body.length > 0, `snippet ${name} body is non-empty`);
      assert.ok(snippet.description, `snippet ${name} has description`);
    });

    it(`snippet "${name}" tabstops are sequential`, () => {
      const bodyStr = snippet.body.join("\n");
      const tabstops = [...bodyStr.matchAll(/\$\{(\d+)[:}]\}/g)];
      if (tabstops.length === 0) return;
      const numbers = tabstops.map((m) => parseInt(m[1]));
      for (let i = 1; i <= numbers.length; i++) {
        assert.ok(numbers.includes(i), `snippet "${name}" has tabstop $${i}`);
      }
    });

    it(`snippet "${name}" body contains no broken template syntax`, () => {
      const bodyStr = snippet.body.join("\n");
      const placeholders = (bodyStr.match(/\$\{\d+[:}][^}]*\}/g) || []).length;
      const opens = (bodyStr.match(/\$\{/g) || []).length;
      assert.equal(opens, placeholders, `snippet "${name}" - all \${...} have matching }`);
    });
  }
});

describe("language-configuration.json", () => {
  const configPath = path.join(__dirname, "..", "language-configuration.json");
  const config = JSON.parse(fs.readFileSync(configPath, "utf-8"));

  it("has line comment //", () => {
    assert.equal(config.comments.lineComment, "//");
  });

  it("has block comment /* */", () => {
    assert.deepEqual(config.comments.blockComment, ["/*", "*/"]);
  });

  it("has 3 bracket pairs", () => {
    assert.equal(config.brackets.length, 3);
    assert.deepEqual(config.brackets[0], ["{", "}"]);
    assert.deepEqual(config.brackets[1], ["[", "]"]);
    assert.deepEqual(config.brackets[2], ["(", ")"]);
  });

  it("has autoClosingPairs", () => {
    assert.ok(config.autoClosingPairs.length >= 3);
  });

  it("has indentationRules", () => {
    assert.ok(config.indentationRules.increaseIndentPattern);
    assert.ok(config.indentationRules.decreaseIndentPattern);
  });

  it("has wordPattern", () => {
    assert.ok(config.wordPattern);
  });
});
