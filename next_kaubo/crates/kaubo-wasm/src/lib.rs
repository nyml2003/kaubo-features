//! Kaubo WASM bindings — compile and run Kaubo code in the browser

use wasm_bindgen::prelude::*;
use std::sync::Mutex;

/// Shared chunk storage — compile deposits, run withdraws
static COMPILED: Mutex<Option<kaubo_ir::Chunk>> = Mutex::new(None);

/// Initialize panic hook so errors show in browser console instead of `unreachable`
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Compile Kaubo source code, store chunk in memory.
/// Returns number of bytecode instructions (for display).
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<u32, JsValue> {
    let owned_source = source.to_owned();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let src = &owned_source;
        let module = kaubo_compiler::ParseStage::new()
            .run(src)
            .map_err(|e| JsValue::from_str(&e))?;

        kaubo_compiler::CheckStage::new()
            .run(&module)
            .map_err(|e| JsValue::from_str(&e))?;

        let chunk = kaubo_compiler::CodegenStage::new()
            .run(&module)
            .map_err(|e| JsValue::from_str(&e))?;

        let len = chunk.code.len() as u32;
        *COMPILED.lock().unwrap() = Some(chunk);
        Ok(len)
    }));

    match result {
        Ok(r) => r,
        Err(panic) => {
            let msg = panic.downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Internal compiler error".to_string());
            Err(JsValue::from_str(&format!("Panic: {msg}")))
        }
    }
}

/// Run the most recently compiled chunk, returns stdout output
#[wasm_bindgen]
pub fn run(_bytes: &[u8]) -> Result<String, JsValue> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        use kaubo_runtime::vm::VmRuntime;
        use std::sync::{Arc, Mutex as StdMutex};

        let chunk = COMPILED.lock()
            .map_err(|_| JsValue::from_str("Lock poisoned"))?
            .take()
            .ok_or_else(|| JsValue::from_str("No compiled chunk — run compile() first"))?;

        let mut vm = kaubo_ir::VM::new();
        vm.init_stdlib();

        let output = Arc::new(StdMutex::new(String::new()));
        let out = output.clone();
        vm.set_output_callback(move |s: &str| {
            out.lock().unwrap().push_str(s);
        });

        vm.interpret(&chunk);

        let s = output.lock().unwrap().clone();
        Ok(s)
    }));

    match result {
        Ok(r) => r,
        Err(panic) => {
            let msg = panic.downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Internal runtime error".to_string());
            Err(JsValue::from_str(&format!("Panic: {msg}")))
        }
    }
}
