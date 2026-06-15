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
    description: "Basic print statement",
    code: '// Hello World\nprint("Hello, Kaubo!");\n',
    tags: ["basics"],
  },
  {
    id: "variables",
    name: "Variables & Types",
    description: "Variable declarations with type annotations",
    code:
      "var name = \"Kaubo\";\n" +
      "var count: int = 42;\n" +
      "var price: float = 9.99;\n" +
      "var active: bool = true;\n" +
      'print(name);\n' +
      "print(count);\n" +
      "print(price);\n" +
      "print(active);\n",
    tags: ["basics", "types"],
  },
  {
    id: "control-flow",
    name: "Control Flow",
    description: "if/elif/else, while, and for-in loops",
    code:
      '// If statement\n' +
      'var x = 10;\n' +
      'if x > 5 {\n' +
      '    print("x is greater than 5");\n' +
      '} elif x == 5 {\n' +
      '    print("x is 5");\n' +
      '} else {\n' +
      '    print("x is less than 5");\n' +
      '}\n' +
      '\n' +
      '// While loop\n' +
      'var i = 0;\n' +
      'while i < 3 {\n' +
      '    print(i);\n' +
      '    i = i + 1;\n' +
      '}\n' +
      '\n' +
      '// For-in loop\n' +
      'for item in [1, 2, 3] {\n' +
      '    print(item);\n' +
      '}\n',
    tags: ["basics", "control-flow"],
  },
  {
    id: "functions",
    name: "Functions & Closures",
    description: "Lambda expressions and closures",
    code:
      '// Lambda with parameters\n' +
      'var add = |a, b| {\n' +
      '    return a + b;\n' +
      '};\n' +
      'print(add(2, 3));\n' +
      '\n' +
      '// Lambda with type annotations\n' +
      'var multiply: |int, int| -> int = |a, b| -> int {\n' +
      '    return a * b;\n' +
      '};\n' +
      'print(multiply(4, 5));\n' +
      '\n' +
      '// Closure capturing outer variable\n' +
      'var makeCounter = || {\n' +
      '    var count = 0;\n' +
      '    return || {\n' +
      '        count = count + 1;\n' +
      '        return count;\n' +
      '    };\n' +
      '};\n' +
      'var counter = makeCounter();\n' +
      'print(counter());\n' +
      'print(counter());\n',
    tags: ["functions", "closures"],
  },
  {
    id: "structs",
    name: "Structs",
    description: "Struct definition and field access",
    code:
      "struct Point {\n" +
      "    x: int,\n" +
      "    y: int,\n" +
      "}\n" +
      "\n" +
      "var p = Point { x: 10, y: 20 };\n" +
      "print(p.x);\n" +
      "print(p.y);\n" +
      "\n" +
      "struct Person {\n" +
      "    name: string,\n" +
      "    age: int,\n" +
      "}\n" +
      "\n" +
      'var alice = Person { name: "Alice", age: 30 };\n' +
      "print(alice.name);\n",
    tags: ["structs"],
  },
  {
    id: "impl",
    name: "Operator Overloading",
    description: "impl block with custom operators",
    code:
      "struct Vector {\n" +
      "    x: int,\n" +
      "    y: int,\n" +
      "}\n" +
      "\n" +
      "impl Vector {\n" +
      "    operator add(other: Vector) -> Vector {\n" +
      "        return Vector { x: self.x + other.x, y: self.y + other.y };\n" +
      "    }\n" +
      "\n" +
      "    operator sub(other: Vector) -> Vector {\n" +
      "        return Vector { x: self.x - other.x, y: self.y - other.y };\n" +
      "    }\n" +
      "}\n" +
      "\n" +
      "var v1 = Vector { x: 1, y: 2 };\n" +
      "var v2 = Vector { x: 3, y: 4 };\n" +
      "var v3 = v1 + v2;\n" +
      "print(v3.x);\n" +
      "print(v3.y);\n",
    tags: ["structs", "operators"],
  },
  {
    id: "lists",
    name: "Lists",
    description: "List creation and built-in methods",
    code:
      'var items = [10, 20, 30];\n' +
      'print(len(items));\n' +
      "\n" +
      "// Iterate over list\n" +
      "for item in items {\n" +
      "    print(item);\n" +
      "}\n" +
      "\n" +
      "// List methods\n" +
      "var letters = [\"a\", \"b\"];\n" +
      'push(letters, "c");\n' +
      "print(len(letters));\n" +
      "\n" +
      "var nums = range(0, 5);\n" +
      "print(length(nums));\n",
    tags: ["lists", "builtins"],
  },
  {
    id: "coroutines",
    name: "Coroutines",
    description: "yield and resume for cooperative multitasking",
    code:
      "var coro = create_coroutine(|| {\n" +
      '    print("coroutine start");\n' +
      "    yield 1;\n" +
      '    print("after first yield");\n' +
      "    yield 2;\n" +
      '    print("after second yield");\n' +
      "});\n" +
      "\n" +
      "var result = resume(coro);\n" +
      "print(result);\n" +
      "result = resume(coro);\n" +
      "print(result);\n",
    tags: ["coroutines"],
  },
  {
    id: "builtins",
    name: "Built-in Methods",
    description: "Common built-in functions: print, type, assert, len",
    code:
      'print("=== Built-in methods ===\\n");\n' +
      "\n" +
      "// Type checking\n" +
      'print(typeof(42));\n' +
      'print(typeof("hello"));\n' +
      'print(typeof(true));\n' +
      "\n" +
      "// Assertions\n" +
      "assert(1 + 1 == 2);\n" +
      'print("assert passed")\n' +
      "\n" +
      "// Math functions\n" +
      "print(sqrt(16));\n" +
      "print(sin(PI / 2));\n" +
      "print(cos(0));\n" +
      "print(floor(3.14));\n" +
      "print(ceil(3.14));\n" +
      "\n" +
      "// String functions\n" +
      "var msg = \"hello kaubo\";\n" +
      "print(to_upper(msg));\n" +
      "print(to_lower(msg));\n" +
      "print(length(msg));\n",
    tags: ["builtins"],
  },
  {
    id: "json",
    name: "JSON Literals",
    description: "JSON-like object notation",
    code:
      "var data = json {\n" +
      '    name: "Alice",\n' +
      "    age: 30,\n" +
      "    scores: [95, 87, 91],\n" +
      '    address: json {\n' +
      '        city: "NYC",\n' +
      '        zip: 10001\n' +
      "    }\n" +
      "};\n" +
      "\n" +
      "print(data.name);\n" +
      "print(data.scores[0]);\n" +
      "print(data.address.city);\n",
    tags: ["json", "data"],
  },
];
