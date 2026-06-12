# Kaubo 交付产物

> v0.2.0 目标 · 2026-06-12

---

## 一、总表

| 产物 | 类型 | 生产者 | 消费者 | 分发方式 |
|------|------|--------|--------|---------|
| `.kaubod` | binary file (debug) | compiler | runtime | 本地编译产出 |
| `.kaubor` | binary file (release) | compiler | runtime | 本地编译 / 随资源下发 |
| `libkaubo_ir.a` | static lib | ir crate | compiler + runtime | link 时 |
| `libkaubo_compiler.a` | static lib | compiler crate | CLI / wasm / embed | link 时 |
| `libkaubo_runtime.a` | static lib | runtime crate | 所有端 | link 时 |
| `kaubo` / `kaubo.exe` | binary | CLI target | 开发者 | `cargo install` / 下载 |
| `kaubo_compiler.wasm` | wasm module | compiler crate | web playground | CDN |
| `kaubo_runtime.wasm` | wasm module | runtime crate | web / edge | CDN |
| `kaubo.h` + `kaubo.dll/so/dylib` | C header + shared lib | ffi target | C/C++/Unity/Unreal | SDK |
| `.kaubor` (嵌入) | binary resource | compiler | game/mobile runtime | 随游戏打包 |

---

## 二、中间产物（编译器线）

### .kaubod — 调试字节码

```
┌──────────────────────────────────────────────────────────────┐
│ .kaubod (Debug Build Mode)                                   │
│                                                              │
│ Header                                                       │
│   magic:    "KAUB" (4 bytes)                                 │
│   version:  u16                                              │
│   build_mode: Debug                                          │
│                                                              │
│ Sections                                                     │
│   StringPool   : 函数名、源码路径                             │
│   FunctionPool : 函数序列化 (每个函数一个 Chunk)              │
│   ShapeTable   : struct Shape 定义                            │
│   ModuleTable  : 模块索引 (名字 → chunk offset)               │
│   ChunkData    : 字节码 + 常量池 + SourceMap                  │
│   DebugInfo    : 完整 LineTable + LocalNameTable              │
│   SourceMap    : ip → (line, col, source_path)               │
│                                                              │
│ 特点: 无压缩，完整调试信息，体积较大                            │
│ 用途: 开发调试，`--dump-bytecode` 可查看                      │
│ 命令: kaubo --emit-binary                                     │
└──────────────────────────────────────────────────────────────┘
```

### .kaubor — 发布字节码

```
┌──────────────────────────────────────────────────────────────┐
│ .kaubor (Release Build Mode)                                 │
│                                                              │
│ Header                                                       │
│   magic:    "KAUB"                                           │
│   build_mode: Release                                        │
│                                                              │
│ Sections                                                     │
│   StringPool   : 去重字符串 (函数名等)                        │
│   FunctionPool : zstd 压缩的函数序列化                        │
│   ShapeTable   : struct Shape 定义                            │
│   ModuleTable  : 模块索引                                    │
│   ChunkData    : zstd 压缩的字节码 + 常量池                   │
│   DebugInfo    : 剥离 (strip) 或不剥离 (可选)                 │
│                                                              │
│ 特点: zstd 压缩，可选剥离调试信息，体积较小                     │
│ 用途: 发布部署，嵌入游戏/App                                  │
│ 命令: kaubo --emit-binary --production                        │
└──────────────────────────────────────────────────────────────┘
```

---

## 三、库产物（link 用）

### libkaubo_ir.a — 共享类型库

```
目标平台: linux, macos, windows (x86_64 / aarch64) + wasm32-unknown-unknown
依赖: 零
no_std: 支持

导出:
  - Value, Chunk, OpCode, InterpretResult
  - ObjString, ObjList, ObjFunction, ObjClosure, ObjStruct, ObjShape
  - ObjCoroutine, ObjIterator, ObjJson, ObjModule
  - CallFrame, InlineCacheEntry, SourceMap

编译: rustc --crate-type=staticlib crates/kaubo-ir/src/lib.rs
```

### libkaubo_compiler.a — 编译器库

```
目标平台: linux, macos, windows + wasm32
依赖: kaubo-ir
no_std: 不支持 (需要 alloc + 字符串处理)

导出:
  - lex(src: &str) -> TokenStream
  - parse(tokens: TokenStream) -> Result<Module>
  - check(ast: &Module) -> Result<TypedModule>
  - lower(typed: &TypedModule) -> Result<HirModule>
  - optimize(hir: HirModule) -> Result<HirModule>
  - compile(hir: &HirModule) -> Result<Chunk>
  - write_binary(chunk: &Chunk, mode: BuildMode) -> Result<Vec<u8>>

编译: rustc --crate-type=staticlib crates/kaubo-compiler/src/lib.rs
```

### libkaubo_runtime.a — 运行时库

```
目标平台: linux, macos, windows + wasm32
依赖: kaubo-ir, kaubo-platform trait
no_std: 不支持 (需要 alloc)

导出:
  - interpret(chunk: &Chunk) -> InterpretResult
  - read_binary(bytes: &[u8]) -> Result<Chunk>
  - RuntimeBuilder { platform, allocator } → build()

编译: rustc --crate-type=staticlib crates/kaubo-runtime/src/lib.rs
```

### kaubo_compiler.wasm / kaubo_runtime.wasm

```
目标: wasm32-unknown-unknown
接口: wasm-bindgen 生成的 JS 函数

compiler wasm:
  JS → kaubo_compile(source_text) → Uint8Array (.kaubor bytes)

runtime wasm:
  JS → kaubo_run(kaubor_bytes) → String (output)

结合:
  // 浏览器 playground
  const compiler = await import("./kaubo_compiler.wasm");
  const runtime = await import("./kaubo_runtime.wasm");
  const bytes = compiler.compile(code);       // src → .kaubor
  const output = runtime.execute(bytes);      // .kaubor → output
```

### kaubo.h + kaubo.dll/so/dylib — C ABI 导出

```
目标: 全平台 (Windows .dll, Linux .so, macOS .dylib, iOS .framework, Android .so)
接口: C ABI, 无 name mangling

kaubo.h:
  typedef struct KauboRuntime KauboRuntime;
  KauboRuntime* kaubo_runtime_create(void);
  void          kaubo_runtime_destroy(KauboRuntime*);
  int           kaubo_runtime_load_bytes(KauboRuntime*, const uint8_t* data, size_t len);
  const char*   kaubo_runtime_execute(KauboRuntime*);
  void          kaubo_runtime_free_string(const char*);
  int           kaubo_runtime_execute_embedded(KauboRuntime*, int script_id);

  使用示例 (C):
    #include "kaubo.h"
    KauboRuntime* rt = kaubo_runtime_create();
    kaubo_runtime_load_bytes(rt, compiled_kaubor_bytes, compiled_kaubor_len);
    const char* output = kaubo_runtime_execute(rt);
    printf("%s\n", output);
    kaubo_runtime_free_string(output);
    kaubo_runtime_destroy(rt);

  使用示例 (C#, Unity):
    [DllImport("kaubo")]
    static extern IntPtr KauboRuntimeCreate();
    [DllImport("kaubo")]
    static extern int KauboRuntimeLoadBytes(IntPtr rt, byte[] data, int len);
    [DllImport("kaubo")]
    static extern IntPtr KauboRuntimeExecute(IntPtr rt);
    // ... string marshal ...
```

---

## 四、端产物（最终二进制）

| 产物 | 平台 | 内容 | 体积(估) | 场景 |
|------|------|------|---------|------|
| `kaubo` | CLI (linux/mac/win) | compiler + runtime + native platform | ~10 MB | 开发者本地编译、运行 |
| `kaubo-playground/` | Web | compiler.wasm + runtime.wasm + index.html | ~600 KB | 在线 Playground |
| `libkaubo_bevy.a` | linux/mac/win | runtime only | ~200 KB | 嵌入 bevy 游戏 |
| `libkaubo_godot.gdext` | linux/mac/win | runtime only | ~200 KB | 嵌入 godot 游戏 |
| `kaubo.aar` | Android | runtime + JNI bindings | ~300 KB | 嵌入 Android App |
| `Kaubo.xcframework` | iOS/macOS | runtime + Swift bindings | ~300 KB | 嵌入 iOS App |
| `kaubo_edge.wasm` | wasm32 (edge) | runtime only | ~80 KB | Cloudflare Workers |
| `kaubo_server.dll/so` | linux/mac/win | runtime + HTTP bindings | ~500 KB | 嵌入 axum/actix 服务 |

### 典型嵌入用例

```rust
// Bevy: 启动时加载预编译脚本
use kaubo_runtime::RuntimeBuilder;
use bevy::prelude::*;

fn load_kaubo_scripts(mut commands: Commands) {
    let rt = RuntimeBuilder::new()
        .platform(Arc::new(BevyPlatform))
        .build();

    let chunk = kaubo_runtime::read_binary(include_bytes!("scripts/main.kaubor")).unwrap();
    rt.execute(&chunk).unwrap();
}
```

```rust
// 服务端: 每个请求执行不同脚本
use axum::{Router, routing::post, Json};
use kaubo_runtime::RuntimeBuilder;
use std::sync::Arc;

async fn run_script(
    State(rt): State<Arc<Runtime>>,
    Json(body): Json<RunScriptBody>,
) -> Json<RunScriptResult> {
    let chunk = kaubo_runtime::read_binary(&body.bytecode).unwrap();
    let result = rt.execute(&chunk);
    Json(RunScriptResult { output: format!("{:?}", result) })
}
```

---

## 五、产物 × 端对照

```
│ 端              │ .kaubor │ libcompiler │ libruntime │ libffi │ wasm │ 部署方式           │
│────────────────│─────────│─────────────│────────────│────────│──────│───────────────────│
│ CLI (开发者)     │ ✅ 产出 │ ✅          │ ✅         │ ❌     │ ❌   │ cargo install     │
│ Web Playground  │ ✅ 产出 │ ✅ (wasm)   │ ✅ (wasm)  │ ❌     │ ✅   │ CDN               │
│ Bevy 游戏       │ ✅ 嵌入 │ ❌          │ ✅         │ ❌     │ ❌   │ 随游戏分发         │
│ Godot 游戏      │ ✅ 嵌入 │ ❌          │ ✅         │ ❌     │ ❌   │ .gdextension bundle│
│ Android App     │ ✅ 嵌入 │ ❌          │ ✅ (JNI)   │ ❌     │ ❌   │ .aar              │
│ iOS App         │ ✅ 嵌入 │ ❌          │ ✅ (Swift) │ ❌     │ ❌   │ .xcframework      │
│ Cloudflare Edge │ ✅ 下发 │ ❌          │ ✅ (wasm)  │ ❌     │ ✅   │ wrangler deploy   │
│ Rust 项目       │ ✅ 嵌入 │ ✅ (可选)   │ ✅         │ ❌     │ ❌   │ Cargo.toml        │
│ C/C++ 项目      │ ✅ 嵌入 │ ❌          │ ❌         │ ✅     │ ❌   │ kaubo.h + .dll/so │
│ Unity 游戏      │ ✅ 嵌入 │ ❌          │ ❌         │ ✅     │ ❌   │ kaubo.h + .dll    │
│ HTTP 服务       │ ✅ 嵌入 │ ❌          │ ✅         │ ❌     │ ❌   │ 随服务部署        │
```

---

## 六、产物关系图

```
                         src.kaubo
                             │
                    ┌────────▼────────┐
                    │  kaubo-compiler │ ───────→ libkaubo_compiler.a
                    └────────┬────────┘          libkaubo_compiler.wasm
                             │
                    ┌────────▼────────┐
                    │    .kaubor      │ ───────→ 磁盘文件 (可持久化)
                    └───────┬─────────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
    ┌───────▼──────┐ ┌──────▼──────┐ ┌──────▼──────┐
    │  CLI 运行     │ │ WASM 运行   │ │  embed 运行  │
    │ load(.kaubor) │ │ fetch + load│ │ include_bytes│
    └───────┬──────┘ └──────┬──────┘ └──────┬──────┘
            │               │               │
    ┌───────▼──────┐ ┌──────▼──────┐ ┌──────▼──────┐
    │ 终端输出      │ │ 浏览器 DOM  │ │ 游戏/App逻辑 │
    └──────────────┘ └─────────────┘ └─────────────┘

    所有端共用的 Runtime 接口:
    ┌───────────────────────────────────────────────┐
    │  kaubo_runtime::execute(&Chunk) → InterpretResult │
    └───────────────────────────────────────────────┘
```

---

*2026-06-12*
