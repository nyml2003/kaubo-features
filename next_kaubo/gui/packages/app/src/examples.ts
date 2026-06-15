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
    description: "Basic output statement",
    code: 'print "Hello, World!";\n',
    tags: ["basics"],
  },
  {
    id: "variables",
    name: "Variables & Types",
    description: "Integer, float, string, bool, null",
    code:
      "var age = 25;\n" +
      "var pi = 3.14159;\n" +
      'var name = "Kaubo";\n' +
      "var is_valid = true;\n" +
      "var nothing = null;\n" +
      "print name;\n" +
      "print age;\n" +
      "print pi;\n" +
      "print is_valid;\n",
    tags: ["basics", "types"],
  },
  {
    id: "control-flow",
    name: "Control Flow",
    description: "if/elif/else, while, for-in with list",
    code:
      "var result = 0;\n" +
      "var age = 18;\n" +
      "if age >= 18 {\n" +
      '    print "Adult";\n' +
      "    result = result + 1;\n" +
      "} else {\n" +
      '    print "Minor";\n' +
      "}\n" +
      "\n" +
      "var counter = 0;\n" +
      "var sum = 0;\n" +
      "while counter < 5 {\n" +
      "    sum = sum + counter;\n" +
      "    counter = counter + 1;\n" +
      "}\n" +
      "print sum;\n" +
      "\n" +
      "var items = [1, 2, 3, 4, 5];\n" +
      "var total = 0;\n" +
      "for var item in items {\n" +
      "    total = total + item;\n" +
      "}\n" +
      "print total;\n",
    tags: ["basics", "control-flow"],
  },
  {
    id: "functions",
    name: "Functions & Closures",
    description: "Lambda, type annotations, closure capture",
    code:
      "var add = |a, b| -> int {\n" +
      "    return a + b;\n" +
      "};\n" +
      "print add(3, 5);\n" +
      "\n" +
      "var greet = || -> string {\n" +
      '    return "Hello!";\n' +
      "};\n" +
      "print greet();\n" +
      "\n" +
      "var square = |x| -> int {\n" +
      "    return x * x;\n" +
      "};\n" +
      "print square(4);\n" +
      "\n" +
      "var make_counter = || {\n" +
      "    var count = 0;\n" +
      "    return || {\n" +
      "        count = count + 1;\n" +
      "        return count;\n" +
      "    };\n" +
      "};\n" +
      "var counter = make_counter();\n" +
      "print counter();\n" +
      "print counter();\n",
    tags: ["functions", "closures"],
  },
  {
    id: "structs",
    name: "Structs",
    description: "Define, instantiate, access fields",
    code:
      "struct Point {\n" +
      "    x: int,\n" +
      "    y: int,\n" +
      "}\n" +
      "\n" +
      "var p = Point { x: 100, y: 200 };\n" +
      "print p.x;\n" +
      "print p.y;\n",
    tags: ["structs"],
  },
  {
    id: "lists",
    name: "Lists",
    description: "Create, index, mutate, iterate lists",
    code:
      "var numbers = [1, 2, 3, 4, 5];\n" +
      "var fruits = [\"apple\", \"banana\", \"cherry\"];\n" +
      "\n" +
      "print numbers[0];\n" +
      "print fruits[1];\n" +
      "\n" +
      "numbers[0] = 10;\n" +
      "print numbers[0];\n" +
      "\n" +
      "var sum = 0;\n" +
      "for var n in numbers {\n" +
      "    sum = sum + n;\n" +
      "}\n" +
      "print sum;\n",
    tags: ["lists"],
  },
  {
    id: "list-methods",
    name: "List Methods",
    description: "Built-in list methods: push, len, map, filter",
    code:
      "var list = [1, 2, 3];\n" +
      "list.push(4);\n" +
      "list.push(5);\n" +
      "print(list.len());\n" +
      "\n" +
      "var doubled = list.map(|x| { return x * 2; });\n" +
      "print(doubled.len());\n" +
      "\n" +
      "var evens = list.filter(|x| { return x % 2 == 0; });\n" +
      "print(evens.len());\n",
    tags: ["lists", "methods"],
  },
  {
    id: "json",
    name: "JSON Literals",
    description: "JSON object creation and field access",
    code:
      'var obj = json { "name": "Kaubo", "age": 1 };\n' +
      "print obj.name;\n" +
      "obj.age = 2;\n" +
      "print obj.age;\n",
    tags: ["json", "data"],
  },
];
