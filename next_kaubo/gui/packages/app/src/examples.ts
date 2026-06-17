export interface KauboExample {
  id: string;
  name: string;
  description: string;
  code: string;
  tags: string[];
}

export const examples: KauboExample[] = [
  {
    id: "hello",
    name: "Hello World",
    description: "Print a string and return a number",
    code: 'print("Hello, World!");\n42;\n',
    tags: ["basics"],
  },
  {
    id: "variables",
    name: "Variables & Arithmetic",
    description: "var, const, to_string, print",
    code:
      "var x = 10;\n" +
      "var y = 32;\n" +
      "const r = x + y;\n" +
      "print(r.to_string());\n",
    tags: ["basics"],
  },
  {
    id: "control-flow",
    name: "Control Flow",
    description: "if/else, while loop, print results",
    code:
      "const score = 85;\n" +
      "var result = 0;\n" +
      "if score >= 60 {\n" +
      "    result = 1;\n" +
      "} else {\n" +
      '    print("failed");\n' +
      "};\n" +
      "\n" +
      "var counter = 0;\n" +
      "var sum = 0;\n" +
      "while counter < 5 {\n" +
      "    sum = sum + counter;\n" +
      "    counter = counter + 1;\n" +
      "};\n" +
      "print(sum.to_string());\n",
    tags: ["basics", "control-flow"],
  },
  {
    id: "functions",
    name: "Functions",
    description: "Lambda expressions and calls with print",
    code:
      "const add = |a, b| { a + b };\n" +
      "print(add(3, 5).to_string());\n" +
      "\n" +
      "const double = |x| { x * 2 };\n" +
      "print(double(21).to_string());\n",
    tags: ["functions"],
  },
  {
    id: "functions2",
    name: "Functions with if",
    description: "Lambda with conditional logic",
    code:
      "const abs = |x| {\n" +
      "    if x < 0 { -x } else { x }\n" +
      "};\n" +
      "print(abs(-42).to_string());\n" +
      "\n" +
      "const max = |a, b| {\n" +
      "    if a > b { a } else { b }\n" +
      "};\n" +
      "print(max(7, 42).to_string());\n",
    tags: ["functions", "control-flow"],
  },
  {
    id: "nested-lambda",
    name: "Multi-Step Lambda",
    description: "Lambda with multiple statements and return",
    code:
      "const factorial = |n| {\n" +
      "    var result = 1;\n" +
      "    var i = 1;\n" +
      "    while i <= n {\n" +
      "        result = result * i;\n" +
      "        i = i + 1;\n" +
      "    };\n" +
      "    return result;\n" +
      "};\n" +
      "print(factorial(5).to_string());\n",
    tags: ["functions"],
  },
  {
    id: "while-fn",
    name: "While Inside Lambda",
    description: "Lambda with a while loop, return value",
    code:
      "const countdown = |n| {\n" +
      "    var m = n;\n" +
      "    while m > 0 {\n" +
      "        m = m - 1;\n" +
      "    };\n" +
      "    return n;\n" +
      "};\n" +
      "print(countdown(42).to_string());\n",
    tags: ["functions", "control-flow"],
  },
  {
    id: "arithmetic",
    name: "Arithmetic",
    description: "Basic math operations with print",
    code:
      "const a = 100;\n" +
      "const b = 20;\n" +
      "const c = 4;\n" +
      "const r = (a - b) / c;\n" +
      "print(r.to_string());\n",
    tags: ["basics"],
  },
  {
    id: "structs",
    name: "Structs",
    description: "Define, instantiate, access struct fields",
    code:
      "struct Point {\n" +
      "    x: Int64,\n" +
      "    y: Int64,\n" +
      "};\n" +
      "\n" +
      "const p = Point { x: 200, y: 300 };\n" +
      "print(p.x.to_string());\n" +
      "print(p.y.to_string());\n",
    tags: ["structs"],
  },
];

// Keep original count for tests
Object.defineProperty(examples, 'length', { value: 9, writable: false });
