//! `ConsoleHandler` — formats events as text and writes to the diagnostic stream.
//!
//! Uses explicit `#[cfg]` branches for native (`eprintln!`) vs WASM
//! (`web_sys::console::error_1`) instead of relying on wasm-bindgen's
//! implicit `eprintln!` → `console.error` remap (which is an implementation
//! detail outside Kaubo's control).

use kaubo_log::{EventHandler, Severity, ToolchainEvent};

/// Writes formatted events to stderr (native) or `console.error` (WASM).
pub struct ConsoleHandler {
    /// Minimum severity level.  Events below this level are filtered out.
    pub min_level: Severity,
}

impl ConsoleHandler {
    pub fn new(min_level: Severity) -> Self {
        Self { min_level }
    }
}

impl EventHandler for ConsoleHandler {
    fn filter(&self, event: &ToolchainEvent) -> bool {
        match event {
            // Instruction events are trace-level (extremely high volume)
            ToolchainEvent::Vm(kaubo_log::VmEvent::Instruction { .. }) => {
                self.min_level <= Severity::Trace
            }
            // Other VM events (LoopIteration, LoopNearLimit) are debug-level
            ToolchainEvent::Vm(_) => self.min_level <= Severity::Debug,
            // CPS and Pass events are debug-level
            ToolchainEvent::Cps(_) | ToolchainEvent::Pass(_) => self.min_level <= Severity::Debug,
            // Diagnostic events use their own severity
            ToolchainEvent::Diagnostic { level, .. } => *level >= self.min_level,
        }
    }

    fn handle(&self, event: &ToolchainEvent) {
        let formatted = format_event(event);

        #[cfg(not(target_arch = "wasm32"))]
        {
            eprintln!("{formatted}");
        }

        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&formatted));
        }
    }
}

// ── Formatting helpers ──

fn format_event(event: &ToolchainEvent) -> String {
    match event {
        ToolchainEvent::Vm(e) => format_vm(e),
        ToolchainEvent::Cps(e) => format_cps(e),
        ToolchainEvent::Pass(e) => format_pass(e),
        ToolchainEvent::Diagnostic {
            level,
            stage,
            message,
        } => {
            format!("[{stage} {level:?}] {message}")
        }
    }
}

fn format_vm(event: &kaubo_log::VmEvent) -> String {
    match event {
        kaubo_log::VmEvent::Instruction {
            func,
            ip,
            opcode,
            inst,
        } => {
            format!("[VM fn={func} ip={ip}] op={opcode:#04x} inst={inst:#010x}")
        }
        kaubo_log::VmEvent::LoopIteration {
            func_idx,
            block_id,
            count,
        } => {
            format!("[VM] loop iteration: fn={func_idx} block={block_id} count={count}")
        }
        kaubo_log::VmEvent::LoopNearLimit {
            func_idx,
            block_id,
            count,
            limit,
        } => {
            format!("[VM] loop near limit: fn={func_idx} block={block_id} count={count}/{limit}")
        }
    }
}

fn format_cps(event: &kaubo_log::CpsEvent) -> String {
    match event {
        kaubo_log::CpsEvent::WhileLowered { header, body, exit } => {
            format!("[CPS] while lowered: header=blk{header} body=blk{body} exit=blk{exit}")
        }
        kaubo_log::CpsEvent::BlockCreated { id, param_count } => {
            format!("[CPS] block created: id={id} params={param_count}")
        }
    }
}

fn format_pass(event: &kaubo_log::PassEvent) -> String {
    match event {
        kaubo_log::PassEvent::Started { name } => {
            format!("[PASS] {name} started")
        }
        kaubo_log::PassEvent::Finished { name } => {
            format!("[PASS] {name} finished")
        }
    }
}
