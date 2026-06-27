//! Structured event types for the Kaubo toolchain.
//!
//! Events are organized by subsystem (VM, CPS, Passes) to avoid a central enum
//! becoming a bottleneck for all stage changes.  Precedent: .NET EventSource per
//! component / LLVM DEBUG_TYPE per pass.

/// Severity level for diagnostic events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Severity {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

// ── VM execution events ──

/// Events emitted during VM instruction execution.
#[derive(Debug, Clone)]
pub enum VmEvent {
    /// Every instruction executed (trace level only — extremely high volume).
    Instruction {
        func: usize,
        ip: usize,
        opcode: u8,
        inst: u32,
    },
    /// A loop iteration was counted on a backward jump.
    LoopIteration {
        func_idx: usize,
        block_id: usize,
        count: u64,
    },
    /// Loop iteration count is ≥ 80% of the configured limit.
    LoopNearLimit {
        func_idx: usize,
        block_id: usize,
        count: u64,
        limit: u64,
    },
}

// ── CPS / IR build events ──

/// Events emitted during AST → CPS lowering.
#[derive(Debug, Clone)]
pub enum CpsEvent {
    /// A while/for loop was lowered to CPS blocks.
    WhileLowered {
        header: usize,
        body: usize,
        exit: usize,
    },
    /// A new CPS block was created.
    BlockCreated {
        id: usize,
        param_count: usize,
    },
}

// ── Pass optimization events ──

/// Events emitted during optimization passes.
#[derive(Debug, Clone)]
pub enum PassEvent {
    /// A pass is about to run.
    Started { name: &'static str },
    /// A pass has completed.
    Finished { name: &'static str },
}

// ── Top-level event ──

/// The single event type that all stages emit through.
///
/// Organized as nested enums so that adding a VM feature doesn't require
/// touching CPS/Pass event variants, and vice versa.
#[derive(Debug, Clone)]
pub enum ToolchainEvent {
    Vm(VmEvent),
    Cps(CpsEvent),
    Pass(PassEvent),
    /// Catch-all diagnostic for uncategorized events from any stage.
    Diagnostic {
        level: Severity,
        stage: &'static str,
        message: String,
    },
}
