# 11 — DAG 调度器（下一代编排层）

**状态**：Phase 1 核心调度器已完成（`crates/kaubo-dag`），Phase 2-4 待实现。

## 一、设计哲学

构建一个**运行时无关（Runtime Agnostic）**的异步 DAG 执行引擎，专门用于编排复杂编译任务的依赖关系与执行时序。

**三条核心原则：**

1. **数据驱动，阶段无感知**：调度器只调度 `ArtifactKey`（模块 + 种类字符串），不关心种类代表 AST、Token 还是二进制。词法、语法、语义等所有编译阶段被封装在独立的 Fetcher 中，调度器对此零感知。

2. **事件驱动，零延时调度**：所有任务切换由通道（`futures::channel`）的 Waker 机制驱动，完全没有 `loop + sleep` 的轮询。任务因等待依赖而挂起，当依赖就绪时被瞬时唤醒，中间不存在任何定时器或延时。

3. **平台无关，自建调度器**：调度器核心完全自建，不依赖 Tokio、async-std 或任何第三方运行时。原生环境与 WASM 环境共享同一套调度逻辑，仅通过薄薄一层 `Spawner` 抽象适配底层平台。

---

## 二、核心概念模型

| 概念 | 定义 | 生命周期 |
|:---|:---|:---|
| **ArtifactKey** | `(ModuleId, Kind)` 数据坐标，调度器的唯一寻址单位 | 静态标识，全局唯一 |
| **Artifact** | 类型擦除的数据容器 `(Key, Hash, Arc<dyn Any>)` | 可缓存、可共享、不可变 |
| **Fetcher** | 数据生产者，接收一组 Artifact 输入，产出一个 Artifact 输出 | 执行时创建，完成后销毁 |
| **Builder** | 终端消费者，接收一组 Artifact 输入，产出最终结果（不缓存） | 执行时创建，完成后销毁 |
| **FetcherRegistry** | 工厂注册表，`Kind → FetcherFactory` 的映射 | 启动时初始化，运行时只读 |

### 依赖规则

```
允许的边:
  Fetcher ──depends on──▶ Fetcher   ✅  (Fetcher 链)
  Builder ──depends on──▶ Fetcher   ✅  (Builder 消费 Fetcher)

禁止的边:
  Builder ──depends on──▶ Builder   ❌  (Builder 互相独立)
  Fetcher ──depends on──▶ Builder   ❌  (Fetcher 不感知 Builder)
```

**关键约束：**
- Fetcher 之间通过 ArtifactKey 建立依赖关系，不直接依赖具体类型。
- Builder 之间互不依赖，每个 Builder 是一个独立的入口点。
- 禁止任何全局可变状态，上下文封装在 Fetcher 的局部栈帧中。
- 跨模块数据传递通过只读 Artifact 进行，接收方只能读取，不能修改。

---

## 三、`Kind` 的类型设计

调度器不关心 `Kind` 的含义，但使用裸 `String` 容易拼写错误；使用枚举又限制扩展性。

**结论**：使用 `Kind` newtype 包装字符串，并提供内置常量。

```rust
pub struct Kind(String);

impl Kind {
    pub const SOURCE: &'static str = "Source";
    pub const TOKEN_STREAM: &'static str = "TokenStream";
    pub const AST: &'static str = "Ast";
    pub const SEMANTIC: &'static str = "Semantic";
    pub const CPS: &'static str = "Cps";
    pub const LINKED_CPS: &'static str = "LinkedCps";
    pub const MODULE_GRAPH: &'static str = "ModuleGraph";

    pub fn custom(s: impl Into<String>) -> Self {
        Kind(s.into())
    }
}
```

**理由**：
- 调度器层将 `Kind` 视为不透明标识符，不关心其含义。
- 用户注册新 Fetcher 时可以使用自定义 `Kind`，无需修改调度器源码。
- 内置常量避免拼写错误。
- 既保持了扩展性，又提供了基础的编译期约束。

**未来增强**（Phase 2+）：`Kind` 可以携带一个可选的"类别标签"（如 `Source`、`Transform`、`Aggregate`），用于调度器层面的优先级或资源分组。标签不参与相等比较和哈希，仅作为调度提示：

```rust
pub struct Kind {
    name: String,
    /// 可选调度提示：Source = I/O 密集型，Transform = CPU 密集型
    hint: Option<KindHint>,
}
```

---

## 四、调度器核心架构

### 4.1 核心数据结构

调度器内部维护四张表 + 一个版本计数器：

| 数据结构 | 用途 |
|:---|:---|
| **Ready Cache** | 已完成的任务结果（Artifact），用于缓存命中 |
| **InFlight Map** | 正在执行的任务，用于去重（相同 Key 只执行一次） |
| **Reverse Dependents** | 反向依赖图，记录"谁在等待谁"，用于缓存失效传播 |
| **Call Stack** | 当前调用链（`Vec` + `HashSet` 双索引），用于循环检测 |
| **Global Epoch** | `AtomicU64` 版本计数器，用于 O(1) 逻辑删除 |

**缓存失效的 Epoch 机制**：

外部触发 `invalidate(key)` 时：
1. 递增 Global Epoch，得到当前版本号 N。
2. 将目标 Key 标记为 "已失效 @ epoch N"（逻辑删除，O(1)）。
3. 新请求到来时，其依赖链中任一 Key 的失效标记 epoch > 请求缓存的 epoch → 自动走重算路径。
4. 旧版本 Artifact 在无 `Arc` 引用后被自动回收。

相比遍历反向依赖图逐项清除的 O(N) 方案，Epoch 机制使失效操作的延迟为常数级，同时避免了清除过程中与新请求之间的竞态窗口。

### 4.2 动态图展开机制（核心调度流程）

调度器不预先构建 DAG，而是采用**惰性求值、按需展开**的策略：

1. 用户发起请求，调度器创建根任务。
2. 任务执行中，调用 `request_dependency(key)` 请求数据：
   - 查 Ready Cache → 命中则立即返回 Artifact。
   - 查 InFlight Map → 命中则挂起当前任务，订阅该 Key 的广播通道。
   - 均未命中 → 通过 FetcherRegistry 创建新 Fetcher 实例，启动执行。
3. 子任务完成后，通过广播通道唤醒所有等待者，结果写入 Ready Cache。
4. 根任务完成，流式输出最终结果。

**关键特性：**
- 运行时发现新依赖（动态 import），调度器自动插入新节点。
- 相同 Key 的并发请求自动去重，只执行一次计算。
- 通过调用栈染色实现循环检测。

### 4.3 任务唤醒机制（零延时）

- 每个 InFlight 任务关联一个 `broadcast::Sender`。
- 等待者调用 `broadcast::Receiver::recv().await` 挂起，Waker 自动注册到通道。
- 任务完成时调用 `broadcast::Sender::send()`，所有等待者被瞬时唤醒。
- **整个过程不涉及任何定时器或轮询。**

### 4.4 取消机制

- 每个任务关联一个 `CancellationToken`（基于 `Arc<AtomicBool>`）。
- 当用户取消请求时（如 LSP 新请求覆盖旧请求），调度器触发取消令牌。
- Fetcher 在每次 `await` 点检查取消状态，检测到后主动退出。
- 取消信号向下游传播，避免无效计算继续消耗资源。

---

## 五、`request_dependency` 的 API 形态

通过 `FetchContext` 透传，采用 `ctx.request_dependency(key).await` 的形式。

```rust
/// FetchContext 内部使用 `Arc<DagScheduler>` 而非 `&'a DagScheduler`，
/// 避免生命周期参数困扰（后台 `'static` 任务可直接持有 clone 后的 Arc）。
pub struct FetchContext {
    scheduler: Arc<DagScheduler>,
    /// 调用栈——`Vec` 保持顺序用于生成循环错误信息，
    /// `HashSet` 提供 O(1) 的成员检测。
    call_stack: Vec<ArtifactKey>,
    call_stack_set: HashSet<ArtifactKey>,
    cancel: CancellationToken,
    /// 双通道：进度事件（有界，可能丢弃） + 结果事件（有界，永不丢弃）
    event_tx: mpsc::Sender<ArtifactEvent>,
}

impl FetchContext {
    pub async fn request_dependency(
        &mut self,
        key: ArtifactKey,
    ) -> Result<Artifact, BuildError> {
        // 1. 循环检测（O(1) 成员检测 + O(N) 错误信息生成）
        if self.call_stack_set.contains(&key) {
            let pos = self.call_stack.iter().position(|k| k == &key).unwrap();
            let cycle = self.call_stack[pos..].to_vec();
            return Err(BuildError::CircularDependency { cycle });
        }
        // 2. 入栈（双写）
        self.call_stack.push(key.clone());
        self.call_stack_set.insert(key.clone());
        // 3. 调用调度器的内部方法
        let result = self.scheduler.request_dependency(key).await;
        // 4. 出栈（双删）
        self.call_stack.pop();
        self.call_stack_set.remove(&key);
        result
    }
}
```

**循环检测**：使用 `Vec` + `HashSet` 双索引结构——成员检测 O(1)，错误信息生成时从 `Vec` 切片。比较的是完整的 `ArtifactKey`（包括 `Kind`），所以 `Semantic(A)` 和 `Source(A)` 是不同的 Key，不会误判为循环。

**多次调用**：Fetcher 内部可以多次调用 `request_dependency`（例如 `SemanticFetcher` 循环请求多个 import 模块的 Semantic）。

**`Arc` 的必要性**：`&'a DagScheduler` 的生命周期参数会污染所有持有 `FetchContext` 的类型，导致后台 `'static` 任务无法持有上下文。使用 `Arc<DagScheduler>` 消除了这个限制。

---

## 六、`is_final` 流式句柄的机制

### 6.1 生命周期

**SourceFetcher 执行：**
1. 创建 `mpsc::channel`。
2. 启动后台任务持续下载数据，不断向 `tx` 发送数据块。
3. 立即返回 `Artifact { data: rx_handle, is_final: false }`。
4. 此时 `request_dependency` 返回，下游被唤醒，**不等待下载完成**。

**下游 Fetcher（如 AstFetcher）：**
1. 拿到 `Artifact.data` 作为句柄（`Stream` 或 `AsyncRead`）。
2. 直接在句柄上拉取数据——**不经过调度器**。
3. 如果数据尚未到达，句柄的 `poll` 返回 `Pending`，当前任务挂起。
4. 后台任务发送数据时，Waker 被触发，任务唤醒继续拉取。

**后台任务下载完成：**
1. 关闭 `tx`（或发送 `Eof` 信号）。
2. 通知调度器标记 `is_final = true`。
3. 调度器将该 Artifact 从 InFlight Map 移入 Ready Cache。
4. 广播唤醒所有正在等待该 Key 的任务。

### 6.2 关键点

- `is_final = false` 的 Artifact **不进入 Ready Cache**，存放在 **InFlight Map** 中。
- 下游通过 `request_dependency` 拿到 Artifact 后，直接拉取通道数据——**绕过调度器**。
- 背压：下游消费慢 → 通道积压 → 后台任务 `tx.send().await` 挂起 → 下载暂停。
- 调度器只在 `is_final = true` 时才做缓存和广播。

### 6.3 状态转换

```
SourceFetcher
  ├─ 1. 创建 mpsc channel
  ├─ 2. 后台 task 持续下载 → tx.send(chunk)
  ├─ 3. Artifact = SourceHandle { rx }
  ├─ 4. 立即返回 Done(artifact, is_final=false)  ← 下游可以开始拉了
  ├─ 5. 后台下载线程继续...
  └─ 6. 下载完毕 → 通知调度器 → 标记 is_final=true → 写入 Ready Cache
```

### 6.4 监护与超时

- 调度器维护一个**监护任务（Watchdog）**，定期扫描 InFlight Map（扫描间隔 ~5s）。
- 每个 InFlight 条目记录创建时间戳，超过配置的超时时间（默认 30s）且未标记 `is_final` 的条目被自动清理。
- 清理时：触发对应 `CancellationToken` → 后台任务收到取消信号 → 退出并清理资源。
- 如果后台任务已崩溃（`tx` 被 drop），监护任务通过 `rx` 的关闭状态检测并移除。

**竞态注意**：监护任务删除条目时需获取 InFlight Map 的写锁。如果此时后台任务恰好完成并尝试调用 `notify_final`，通过"先查 InFlight 是否存在该 Key → 存在则操作，不存在则忽略"的检查-然后-操作模式避免竞态。

### 6.5 大对象内存管理

对于 `is_final = true` 的大型 Artifact（如完整 AST、Source 文本），提供两种存储模式：

| 模式 | 说明 | 适用场景 |
|:---|:---|:---|
| **内联存储**（默认） | Artifact 内部持有 `Arc<dyn Any>` | 中小对象，频繁访问 |
| **惰性句柄** | Artifact 仅持有哈希 + 重构闭包（`Arc<dyn Fn() -> Artifact>`） | 大文件、可重新计算的中间产物 |

惰性句柄模式：缓存命中时返回一个"空壳" Artifact，下游首次 `downcast` 时触发重构。内存中只保留高频访问的 Artifact，低频数据按需重建或从磁盘加载。

---

## 七、跨模块依赖的位置与粒度

### 7.1 分层说明

| 层级 | 是否有跨模块边 | 说明 |
|:---|:---|:---|
| Source → TokenStream → Ast | ❌ 无 | 纯词法/语法，不依赖外部信息 |
| Semantic（类型推断） | ✅ 有 | 需要导入模块的类型信息（如常量类型、函数签名） |
| Cps（局部优化） | ❌ 无 | 局部优化不跨模块，在模块本地完成 |
| Link | 天然聚合 | 符号解析 + 跨模块类型检查 + 全局优化 |

### 7.2 Semantic 层行为

`Semantic(A)` 遇到 `import B` 时：
- **记录符号占位符（`SymbolRef`）**，不请求 `Semantic(B)` 的具体类型。
- 本地只做"类型存根（Stub）"生成或基础语法检查。
- 产出包含"未解析符号引用"的 IR。

### 7.3 Link 层行为

`LinkedCpsFetcher` 等待所有 Cps 完成。为提高并行度，采用**两阶段增量合并**：

**阶段 1 — 增量符号合并**（可与最后几个模块的编译重叠）：
- `LinkedCpsFetcher` 的依赖流中，每收到一个模块的 `Cps Done` 事件，就将其符号表增量合并到全局符号图中（函数名、常量名、结构体名）。
- 增量合并是纯追加操作，不需要等待其他模块。

**阶段 2 — 最终检查**（唯一同步屏障，所有模块 Cps 到齐后）：
- 执行全局类型统一/检查。
- 解析所有跨模块引用（`CallExternal → Call`）。
- 如果发现类型不匹配（如 A 将 B 的 Int 当 String 用），抛出 `LinkError`。
- 通过检查后进行全局优化（跨模块内联、全局死代码消除）。

### 7.4 实际 DAG

```
Source(A) → Token(A) → Ast(A) → Semantic(A) → Cps(A) ───────┐
                                                             │
Source(B) → Token(B) → Ast(B) → Semantic(B) → Cps(B) ───────┤
                                                             │
Source(C) → Token(C) → Ast(C) → Semantic(C) → Cps(C) ───────┤
                                                             │
Source(D) → Token(D) → Ast(D) → Semantic(D) → Cps(D) ───────┤
                                                             │
                                             (所有Cps同时就绪) │
                                                             ▼
                                                LinkedCpsFetcher
                                        (符号解析 + 类型统一 + 全局优化)
                                                      │
                                                      ▼
                                                 最终输出
```

### 7.5 理由

- Semantic 阶段**零跨模块阻塞**，N 个模块完全并行。
- 类型变动（如 B 改了签名）不影响 A 的 Semantic 缓存，只在 Link 阶段重新解析。
- 增量编译效率更好：变动仅在 Link 阶段重算，不影响上游编译缓存。
- 如果 A 的 Semantic 确实需要 B 的类型信息（当前 kaubo-infer 的实现），则在 SemanticFetcher 内部通过 `ctx.request_dependency(Semantic(B))` 动态获取。调度器自动处理拓扑约束，四条流水线不再是完全独立的。

### 7.6 增量合并的性能收益

两阶段增量合并的关键优势：当最后一个模块的 Cps 完成时，所有前置模块的符号表已经合并完毕。最终检查阶段只需处理剩余的跨模块引用解析 + 类型统一，而非从头扫描全部模块。当模块数量较大（N > 10）时，增量合并可显著降低 Link 阶段的关键路径延迟。

---

## 八、TokenStream 是否拆成独立 Fetcher

设计上允许拆分，但默认实现建议合并。

**AstFetcher 内部（合并方案）：**
1. 启动一个后台任务：下载 → 词法分析 → 语法分析 → 每完成一个 Ast 节点 → `tx.send(node)`
2. 立即返回 `Artifact { data: rx, is_final: false }`

**对外表现**：`Stream<AstNode>` 流式句柄（`is_final = false`）。
**内部实现**：下载 + 词法 + 语法在同一个后台任务中流水线重叠。

**理由**：对于几百 Token 的小文件，拆成 3 个独立 Fetcher 的调度开销（任务创建、通道通信、广播等待）可能超过实际编译时间。

**灵活性**：如果场景需要，用户可以注册独立的 `TokenFetcher`，从 `Source` 产出 `TokenStream`，然后 `AstFetcher` 依赖 `TokenStream` 而非 `Source`。设计上支持，但不强制。

---

## 九、`Spawner` trait 的精确定义

### 9.1 签名

```rust
/// 平台适配层——调度器通过此 trait 与底层异步运行时交互。
pub trait Spawner: Send + Sync {
    /// 生成一个后台任务。
    /// 返回 `()`，取消由 `CancellationToken` 驱动。
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);

    /// 主动让出当前任务的执行权，允许其他就绪任务运行。
    ///
    /// 语义保证：
    ///   - 调用后，当前任务被挂起并重新排入就绪队列末尾。
    ///   - 其他已就绪的任务有机会在此间隙中执行。
    ///   - 这不是"让出 CPU 核心"（原生 `thread::yield_now`），
    ///     而是"让出协程调度权"。
    ///
    /// 使用场景：
    ///   - WASM 单线程：防止长计算阻塞 UI 渲染和事件处理。
    ///   - 原生环境：在密集型计算循环中插入 yield 点，
    ///     防止单任务独占调度器，使其他并发 Fetcher 获得进展。
    fn yield_now(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;

    /// 创建一个取消令牌。
    fn cancellation_token(&self) -> CancellationToken;

    /// 为当前线程提供阻塞等待 Future 的能力（仅原生环境）。
    ///
    /// WASM 环境下此方法 panic，因为浏览器主线程不允许阻塞。
    #[cfg(not(target_arch = "wasm32"))]
    fn block_on<F: Future>(&self, future: F) -> F::Output;
}
```

### 9.2 原生实现

- `spawn`：线程池包装阻塞 I/O，配合自建调度循环。
- `yield_now`：将当前任务重新排入调度器就绪队列末尾。
- `block_on`：`futures::executor::block_on`，供旧 API 的同步包装使用。
- `CancellationToken`：自建实现（`Arc<AtomicBool>`）。

### 9.3 WASM 实现

- `spawn`：`wasm_bindgen_futures::spawn_local`。
- `yield_now`：`gloo::timers::future::TimeoutFuture::new(0)` 或通过 `Promise` + `setTimeout(0)` 实现。注意这是真正的异步 yield——当前任务被挂起，浏览器事件循环在间隙中运行，之后任务被重新唤醒。
- `block_on`：编译期不存在（`#[cfg]` 排除），WASM 调用方必须使用 async API。
- `CancellationToken`：自建实现（`Arc<AtomicBool>`）。

**关键点**：调度器核心代码完全基于 `Spawner` trait 编写，与具体平台解耦。核心代码在原生和 WASM 环境下完全一致，只有 `Spawner` 的实现通过条件编译切换（约 50 行代码）。

---

## 十、事件流与背压

### 10.1 双通道设计

事件流分为两个独立通道，避免进度事件阻塞结果事件：

| 通道 | 类型 | 策略 | 内容 |
|:---|:---|:---|:---|
| **进度通道** | `mpsc::Sender`，无界或大容量 | **可丢弃**：通道满时 `try_send` 失败则跳过 | `Progress`, `DependencyReady` |
| **结果通道** | `mpsc::Sender`，有界（容量 64） | **可靠**：背压传导，永不丢弃 | `Done`, `Error` |

```rust
pub struct FetchContext {
    /// 进度事件——无界，发送失败直接丢弃（不阻塞生产者）。
    progress_tx: mpsc::UnboundedSender<ProgressEvent>,
    /// 结果事件——有界，发送时若满则挂起（背压传导）。
    result_tx: mpsc::Sender<ResultEvent>,
    // ...
}

pub struct EventReceiver {
    progress_rx: mpsc::UnboundedReceiver<ProgressEvent>,
    result_rx: mpsc::Receiver<ResultEvent>,
}
```

### 10.2 分离的理由

- **进度事件高频**（每个 Fetcher 可能 emit 数十次 `Progress`），且仅用于 UI 展示。如果外部 UI 消费慢，不应阻塞编译管线。
- **结果事件关键**（`Done` / `Error`），丢失意味着编译结果不可知。有界通道 + 背压确保结果不丢失。
- 分离后，外部可以选择只消费结果通道（CLI 场景），或同时消费两个通道（IDE 进度条场景）。

### 10.3 生命周期

1. 调度器为每个顶层请求（`fetch` 或 `build`）创建两对 channel。
2. 所有 Fetcher/Builder 共享同一个 `progress_tx` 和 `result_tx`（克隆）。
3. 外部通过 `EventReceiver` 分别消费进度流和结果流。
4. 事件流中通过 `ArtifactKey` 区分来源（`DependencyReady { key }`）。

### 10.4 事件类型

| 事件类型 | 所属通道 | 触发时机 | 消费端 |
|:---|:---|:---|:---|
| `Progress` | 进度 | 进度更新 | CLI 进度条、LSP 进度推送 |
| `DependencyReady` | 进度 | 依赖就绪 | 调试日志 |
| `Done` | 结果 | 任务完成 | 结果消费 |
| `Error` | 结果 | 任务失败 | 错误报告 |

### 10.5 背压

- 结果通道有界（容量 64），生产者 `send().await` 满时挂起 → 生产速度自动匹配消费速度。
- 进度通道无界，`unbounded_send` 永不被阻塞 → 编译管线不受 UI 消费速度影响。
- 极端情况下进度通道内存持续增长：通过 `EventReceiver` 的 `try_recv` 或批量拉取帮助消费者快速排空。

---

## 十一、缓存策略与增量编译

### 11.1 内容寻址

每个 Artifact 的哈希值：`SHA256(own_data + sorted(dep_keys + dep_hashes))`。

形成传递闭包——任意依赖变化都会导致上游哈希变化，自动失效。

### 11.2 缓存命中与失效

- **命中**：请求 Key 时，对比当前依赖的哈希值与缓存中的哈希值，一致则命中。
- **失效**：外部触发 `invalidate(key)` → 清除该 Key 缓存 → 沿反向依赖图递归清除所有下游 → 取消进行中的相关任务。
- **版本化**：LSP 场景下，Builder 事件携带文档版本号，客户端过滤过期结果。

### 11.3 缓存层级

- **L1 内存缓存**：`DashMap<Key, Artifact>`，当前会话内共享。
- **L2 磁盘缓存**（未来扩展）：基于内容哈希的文件存储，跨会话复用。

---

## 十二、扩展机制

### 12.1 新增 Fetcher

1. 实现 `Fetcher` trait（`key`、`dependencies`、`fetch`）。
2. 在启动时注册到 `FetcherRegistry`：`registry.register("Kind", factory)`。
3. 调度器自动识别并调度该 Fetcher。
4. 无需修改调度器核心代码。

**Registry 并发策略**：

- **初期**：仅支持启动时一次性注册。使用 `OnceCell<HashMap<Kind, Factory>>` 或构造器模式，无需锁。
- **后期**（如需运行时动态注册，如插件系统）：升级为 `RwLock<HashMap<Kind, Factory>>`，读多写少场景性能可接受。

### 12.2 新增 Builder

1. 实现 `Builder<Out>` trait（`name`、`dependencies`、`build`）。
2. 通过调度器 API 直接执行：`scheduler.build(builder)`。
3. 与现有 Builder 共享同一份缓存数据。
4. 无需修改调度器核心代码。

### 12.3 示例扩展

| 扩展需求 | 操作 | 是否改核心 |
|:---|:---|:---|
| 新增编译阶段（字节码生成） | 实现 `BytecodeFetcher`，注册 `"Bytecode"` | 否 |
| 新增终端工具（导出 JSON 类型） | 实现 `JsonSchemaBuilder`，声明依赖 `"Semantic"` | 否 |
| 支持新文件类型（`.json`） | 实现 `JsonSourceFetcher`，注册 `"JsonSource"` | 否 |
| 自定义缓存后端（Redis） | 实现 `CacheBackend` trait，替换存储层 | 否 |
| 调整并发度 | 修改调度器构造参数 | 否 |

---

## 十三、错误处理

### 13.1 编译错误

- Fetcher 执行过程中遇到错误（语法错误、类型错误），通过事件流向上层报告 `BuilderEvent::Error`。
- 调度器取消所有依赖该产物的进行中任务。
- 错误信息携带完整上下文（模块、位置、错误类型）。

### 13.2 链接错误

- `LinkedCpsFetcher` 在合并符号表时发现问题（如 D 依赖 A 但 A 未导出符号）。
- 通过事件流报告 `BuildError::LinkError`。
- 调度器将其作为正常的 Fetcher 失败处理。

### 13.3 取消错误

- 用户主动取消（如 LSP 新请求覆盖旧请求）。
- 任务检测到取消令牌被触发，返回 `BuildError::Cancelled`。
- 调度器清理相关资源，不缓存失败结果。

### 13.4 降级策略

部分场景下，Fetcher 失败不一定需要取消整个编译。例如 LSP 场景中某个模块的 Semantic 推断失败，不应阻止其他模块的语法高亮。

```rust
pub enum ArtifactEvent {
    /// Fetcher 成功完成
    Done { key: ArtifactKey, artifact: Artifact },
    /// Fetcher 失败（不可恢复，取消所有下游）
    Error { key: ArtifactKey, error: Arc<BuildError> },
    /// Fetcher 部分失败（降级产物可用，下游可选择是否继续）
    Degraded { key: ArtifactKey, artifact: PartialArtifact, error: Arc<BuildError> },
}
```

- 上游 Fetcher 可选择返回 `Degraded`，携带一个可用但不完整的 Artifact。
- 下游 Fetcher 收到 `Degraded` 后自行决定：容忍不完整数据继续计算，或将 `Degraded` 升级为 `Error`。
- 实现方式：`Fetcher::execute` 的输出流类型为 `Result<Artifact, BuildError>` 的扩展版本，上游通过特定的 stream item 变体表达降级。

---

## 十四、典型工作流（4 模块并发编译）

### 14.1 场景设定

- 模块 A、B、C、D 四个模块。
- D 依赖 A 和 B，A 和 B 依赖 C（仅在链接期检查，编译期不感知）。
- 所有 Source 来自远程网络，需要下载。

### 14.2 执行时间线

| 阶段 | 发生的事件 | 并发状态 |
|:---|:---|:---|
| **T0** | 用户请求链接，调度器创建 `LinkedCps` 任务 | 根任务启动 |
| **T1** | `LinkedCps` 请求 Cps(A/B/C/D)，触发 4 条独立流水线 | 4 个任务并发 |
| **T2** | 每条流水线：Source → Ast → Semantic → Cps | 流水线内部重叠 |
| **T3** | Source 返回流句柄，Ast 立即启动（不等下载完成） | 边下载边解析 |
| **T4** | 四模块并发编译，无跨模块等待 | 最大化并行 |
| **T5** | 模块陆续完成，Cps 产物写入缓存 | 逐模块就绪 |
| **T6** | 所有 Cps 完成，`LinkedCps` 被唤醒 | 唯一同步屏障 |
| **T7** | 链接器合并符号表，检查依赖，报告结果 | 批处理执行 |

### 14.3 DAG 拓扑

```
模块 A：Source(A) → Ast(A) → Sem(A) → Cps(A) ─┐
模块 B：Source(B) → Ast(B) → Sem(B) → Cps(B) ─┤
模块 C：Source(C) → Ast(C) → Sem(C) → Cps(C) ─┼→ LinkedCps → 结果
模块 D：Source(D) → Ast(D) → Sem(D) → Cps(D) ─┘
```

**关键特征：**
- 四条流水线完全独立，无跨模块边。
- 唯一的汇聚点是 `LinkedCps`，等待所有 Cps 完成。
- 链接阶段的依赖检查在 `LinkedCps` 内部完成，调度器不感知。

---

## 十五、向后兼容与迁移路径

### 15.1 双轨并行

- 现有 `Coordinator` 和 `Stage` 系统保留。
- 新架构在 `dag/` 模块独立实现。
- 旧 API（`compile_source`、`run_source`）内部逐步改为委托给新调度器。

### 15.2 新旧 API 桥接

旧 API 是同步的（`fn compile_source(source: &str) -> Result<CpsModule, DriverError>`），新调度器是异步的。桥接层提供 `block_on` 包装：

```rust
// 原生环境：使用 Spawner::block_on 将异步调用包装为同步
pub fn compile_source(source: &str) -> Result<CpsModule, DriverError> {
    let scheduler = DagScheduler::new(NativeSpawner::new());
    let builder = CompileBuilder::new(source);
    // block_on 仅在原生环境可用
    scheduler.spawner().block_on(async {
        let mut stream = scheduler.build(&builder).await?;
        // 消费 stream 直到 Done
        match stream.next().await {
            Some(BuilderEvent::Done(cps)) => Ok(cps),
            Some(BuilderEvent::Error(e)) => Err(e.into()),
            _ => Err(DriverError::Build("unexpected end of stream".into())),
        }
    })
}

// WASM 环境：仅提供异步 API，block_on 编译期不存在
#[cfg(target_arch = "wasm32")]
pub async fn compile_source_async(source: &str) -> Result<CpsModule, DriverError> {
    // ...
}
```

### 15.3 迁移阶段

| 阶段 | 工作内容 |
|:---|:---|
| **Phase 1** | 引入 `dag/` 模块，实现核心调度器、FetcherRegistry、ArtifactStore |
| **Phase 2** | 将现有 Stage 改写为 Fetcher（Source → Ast → Semantic → Cps → LinkedCps） |
| **Phase 3** | 实现 Builder 层（ExecuteBuilder、LspSnapshotBuilder） |
| **Phase 4** | 旧 API 底层切换到新调度器（通过 `block_on` 桥接），废弃旧 Coordinator |

### 15.4 风险控制

- 每个阶段保持新旧系统共存，逐步切换。
- 基准测试对比新旧系统的性能，确保无回退。
- 单模块编译路径走快速通道，避免 DAG 调度开销。

---

## 十六、目录结构

```
kaubo-driver/src/
├── dag/
│   ├── mod.rs              # 模块入口
│   ├── types.rs            # ArtifactKey, Kind, ContentHash, Artifact
│   ├── fetcher.rs          # Fetcher trait + FetchContext
│   ├── builder.rs          # Builder trait + BuilderEvent
│   ├── store.rs            # ArtifactStore (Ready Cache + InFlight + ReverseDeps)
│   ├── scheduler.rs        # DagScheduler (Stream 图编排核心)
│   ├── spawner.rs          # Spawner trait + NativeSpawner + WasmSpawner
│   ├── cancel.rs           # CancellationToken
│   ├── registry.rs         # FetcherRegistry
│   └── error.rs            # BuildError 扩展
│
├── fetchers/               # 内置 Fetcher 实现
│   ├── source.rs           # SourceFetcher
│   ├── module_graph.rs     # ModuleGraphFetcher
│   ├── ast.rs              # AstFetcher
│   ├── semantic.rs         # SemanticFetcher
│   ├── cps.rs              # CpsFetcher (含 Pass pipeline)
│   └── linked_cps.rs       # LinkedCpsFetcher
│
├── builders/               # 内置 Builder 实现
│   ├── execute.rs          # ExecuteBuilder → RunOutcome
│   └── lsp.rs              # LspSnapshotBuilder
│
├── coordinator.rs          # [保留] 旧 Coordinator，向后兼容
├── protocol.rs             # [保留] Stage/Pass/Pipeline traits
├── stages.rs               # [保留] 旧 Stage 实现
├── module_graph.rs         # [保留] 被 ModuleGraphFetcher 复用
├── module_compiler.rs      # [保留] 逐步废弃
├── module_loader.rs        # [保留] 被 SourceFetcher 复用
├── link_stage.rs           # [保留] 被 LinkedCpsFetcher 复用
├── export_table.rs         # [保留] 不变
└── event.rs                # [保留] 不变
```

---

## 十七、设计决策汇总

| # | 决策点 | 结论 |
|:---|:---|:---|
| 1 | 调度器模型 | 自建调度器，基于 `futures::channel` + `ArcWake` |
| 2 | 图展开方式 | 惰性求值、动态展开 |
| 3 | 依赖 API | `ctx.request_dependency(key).await` |
| 4 | 任务唤醒 | `broadcast::Sender` Waker 驱动，零轮询 |
| 5 | **Kind 类型** | `Kind(String)` newtype + 内置常量，预留调度提示标签 |
| 6 | **is_final 生命周期** | InFlight Map（false）→ 后台完成 → Ready Cache（true）；Watchdog 超时清理 |
| 7 | **跨模块类型解析** | Link 阶段统一做（增量合并 + 最终检查两阶段） |
| 8 | **TokenStream Fetcher** | 可拆可合，默认合并到 AstFetcher 内部 |
| 9 | **Spawner trait** | `spawn`, `yield_now`, `cancellation_token`, `block_on`（仅原生） |
| 10 | **循环检测** | `call_stack` Vec + HashSet 双索引，O(1) 成员检测 |
| 11 | **事件流** | 双通道：进度无界可丢 + 结果有界可靠 |
| 12 | **GC** | 监护任务定期扫描 + 取消驱动清理 + Epoch 逻辑删除 |
| 13 | **最终状态转换** | 后台任务通知调度器 `notify_final`，检查-然后-操作防竞态 |
| 14 | 缓存失效 | 传递闭包哈希 + Global Epoch O(1) 逻辑删除 |
| 15 | 大对象存储 | 内联（默认）或惰性句柄（大文件按需重构） |
| 16 | 并发安全 | 无共享可变状态，Rust 所有权系统保证 |
| 17 | WASM 兼容 | `Spawner` trait 抽象，条件编译；WASM 不支持 `block_on` |
| 18 | 向后兼容 | 双轨并行，原生用 `block_on` 桥接，WASM 仅异步 API |
| 19 | 降级策略 | Fetcher 可返回 `Degraded` 携带不完整数据，下游自行决策 |
| 20 | Registry 并发 | 初期 `OnceCell` 一次性注册，后期可升级 `RwLock` |

---

## 十八、与现有架构对比

| 维度 | 旧 Coordinator | 新 DagScheduler |
|:---|:---|:---|
| 执行模型 | `fn execute(input) -> Result<output>` 同步函数调用 | `fn execute(ctx) -> Stream<Item>` 流式拉取 |
| 多模块 | `for path in order { ... }` 串行 | 所有就绪模块并发启动，Stream 合并 |
| 进度 | 无（只能等最终结果或靠 EventHandler 旁路） | 一等公民：`Progress` 事件沿 Stream 流动 |
| 取消 | 不支持（只能等函数返回或 panic） | `drop(stream)` → `CancellationToken` → 所有 task 终止 |
| 缓存 | 全局 HashMap，手动管理 key | `ArtifactStore` 自动填充，依赖追踪 |
| 依赖声明 | 硬编码在 `build_cps()` 里 | `request_dependency(key).await` 动态按需 |
| 扩展新 Stage | 改 Coordinator 源码 | 实现 `Fetcher` trait + 注册到 `FetcherRegistry` |
| 并行度 | 1（单线程同步） | N（多任务并发） |
| 运行时依赖 | 无 | `futures`（`no_std` 兼容） |
| 向后兼容 | — | 旧 API 内部委托给 DagScheduler |

---

## 十九、评审反馈与修订记录

### 反馈来源

基于系统架构评审，以下 10 项改进已纳入文档。

### 修订清单

| # | 反馈项 | 风险等级 | 修订内容 | 涉及章节 |
|:---|:---|:---|:---|:---|
| 1 | 缓存失效并发一致性 | 高 | 引入 Global Epoch 机制，O(1) 逻辑删除替代级联物理清除 | §4.1, §17 |
| 2 | `is_final` 生命周期管理 | 中 | 新增监护任务（Watchdog）定期扫描超时 InFlight 条目；明确竞态处理策略 | §6.4 |
| 3 | 大对象内存占用 | 中 | 新增惰性句柄存储模式，大 Artifact 按需重构而非长期驻留内存 | §6.5 |
| 4 | `Kind` 扩展性 | 低 | 预留可选调度提示标签（`KindHint`），Phase 2+ 实现 | §3 |
| 5 | 事件流背压传播 | 中 | 拆分为双通道：进度（无界可丢）+ 结果（有界可靠），避免 UI 慢消费阻塞编译 | §10 |
| 6 | Link 阶段性能瓶颈 | 中 | 改为两阶段增量合并（增量符号合并可与最后几个模块编译重叠） | §7.3, §7.6 |
| 7 | 迁移路径 WASM 阻塞 | 高 | 新增 `block_on` 仅原生可用，WASM 提供独立 `_async` API；`Spawner` trait 新增此方法 | §15.2, §9 |
| 8 | `yield_now` 语义模糊 | 低 | 明确定义为"让出协程调度权"而非"让出 CPU 核心"；原生和 WASM 语义统一 | §9.1 |
| 9 | `FetchContext` 生命周期 | 中 | `&'a DagScheduler` 改为 `Arc<DagScheduler>`，消除生命周期参数限制 | §5 |
| 10 | 调用栈检测 O(N²) | 低 | `Vec` + `HashSet` 双索引，成员检测 O(1) | §5 |
| 11 | Registry 并发策略 | 低 | 初期 `OnceCell` 一次性注册，后期可升级 `RwLock` | §12.1 |
| 12 | 降级策略 | 低 | 新增 `Degraded` 事件变体，支持部分失败的降级产物 | §13.4 |

### 测试建议

| 测试类型 | 目标 | 方法 |
|:---|:---|:---|
| 压力测试 | 循环检测 + 去重 | 大量动态 import，验证无死锁无重复计算 |
| 取消测试 | 无内存泄漏 | 高频取消（模拟快速编辑），检查僵尸任务 |
| 缓存一致性 | 级联失效 | 修改一个模块，验证所有下游自动重算 |
| 背压测试 | 内存可控 | 慢消费者场景，观察进度通道内存增长 |
| WASM 兼容 | 浏览器可运行 | 简单编译在浏览器环境中通过 `spawn_local` 执行 |
| 降级测试 | 部分失败可恢复 | 一个模块 Semantic 失败，LSP 仍能提供其他模块的高亮 |
