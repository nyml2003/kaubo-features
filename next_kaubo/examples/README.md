# Kaubo Examples

This directory contains example programs demonstrating various features of the Kaubo language.

## Single-File Examples

### `hello/`
Basic "Hello World" program.

### `calc/`
Simple calculator demonstrating arithmetic operations.

### `fib/`
Fibonacci sequence implementation.

### `builtin_methods.kaubo`
Demonstrates built-in methods for strings and lists.

## Multi-Module Examples

### `multi_module/`
Demonstrates basic multi-file project structure.

**Files:**
- `main.kaubo` - Entry point, imports and uses math and utils
- `math.kaubo` - Math constants and functions (exports PI, E, add, multiply, circle_area)
- `utils.kaubo` - General utilities (exports format_number, is_even, factorial)

**Key Concepts:**
- `import math;` - Import a module
- `pub var` - Export a variable/function from a module
- Accessing exports: `math.PI`, `math.add(1, 2)`

---

### `import_chain/`
Demonstrates transitive dependencies (A → B → C chain).

**Files:**
- `main.kaubo` - Imports app
- `app.kaubo` - Imports database, provides run()
- `database.kaubo` - Imports logger, provides database operations
- `logger.kaubo` - Base module, provides logging functions

**Dependency Chain:**
```
main → app → database → logger
```

**Key Concepts:**
- Transitive dependencies are automatically resolved
- Dependencies are loaded in the correct order (topological sort)

---

### `diamond_deps/`
Demonstrates diamond dependency resolution.

**Files:**
- `main.kaubo` - Imports both math and strings
- `math.kaubo` - Imports common
- `strings.kaubo` - Also imports common
- `common.kaubo` - Shared utilities (VERSION, assert, repeat)

**Dependency Structure:**
```
      main
     /    \
  math    strings
    \      /
     common
```

**Key Concepts:**
- Shared dependencies (common) are loaded only once
- No duplicate loading or circular reference issues

---

### `nested_import/`
Demonstrates nested path imports (e.g., `import std.list`).

**Files:**
- `main.kaubo` - Entry point
- `std/list.kaubo` - List operations (map, filter, reduce, sum)
- `std/math.kaubo` - Math operations (abs, pow, sqrt)
- `app/utils.kaubo` - Application utilities, imports std.math

**Import Syntax:**
```kaubo
import std.list;    // Resolves to std/list.kaubo
import std.math;    // Resolves to std/math.kaubo
import app.utils;   // Resolves to app/utils.kaubo
```

**Key Concepts:**
- Dot notation maps to directory structure
- `import a.b.c` → `a/b/c.kaubo`
- Nested modules can import other nested modules

---

## Module System Overview

In Kaubo:

1. **Single File = Single Module**: Each `.kaubo` file is its own module
2. **No `module` Keyword**: The `module` keyword is deprecated; file organization determines modules
3. **Exports**: Use `pub var` to export values from a module
4. **Imports**: Use `import path.to.module;` to import other modules
5. **Resolution**: Import paths use dots to represent directory structure
6. **Caching**: Modules are loaded only once, even if imported multiple times

### Example Module Structure

```
my_project/
├── main.kaubo          # Entry point
├── math.kaubo          # Math utilities
├── utils/
│   ├── string.kaubo    # String utilities
│   └── io.kaubo        # I/O utilities
└── std/
    ├── list.kaubo      # List operations
    └── json.kaubo      # JSON handling
```

### Import Examples

```kaubo
// Simple import
import math;

// Nested path import  
import utils.string;
import std.list;

// Using imported modules
var result = math.add(1, 2);
var doubled = std.list.map([1, 2, 3], |x| { return x * 2; });
```
