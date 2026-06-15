//! VM execution methods — extension trait on kaubo_ir::VM

use kaubo_ir::{
    CallFrame, Chunk, InlineCacheEntry, InterpretResult, ObjClosure, ObjFunction, ObjShape,
    ObjUpvalue, Operator, Value, VM,
};

pub mod call;
pub mod execution;
pub mod index;
pub mod operators;
pub mod shape;
pub mod stack;

pub use stack::stack_top;
pub use operators::call_operator_closure;

pub trait VmRuntime {
    fn init_stdlib(&mut self);
    fn stack_mut(&mut self) -> &mut Vec<Value>;
    fn frames_mut(&mut self) -> &mut Vec<CallFrame>;
    fn open_upvalues_mut(&mut self) -> &mut Vec<*mut ObjUpvalue>;
    fn interpret(&mut self, chunk: &Chunk) -> InterpretResult;
    fn interpret_with_locals(&mut self, chunk: &Chunk, local_count: usize) -> InterpretResult;
    fn run(&mut self) -> InterpretResult;
    unsafe fn register_shape(&mut self, shape: *const ObjShape);
    fn register_method_to_shape(&mut self, shape_id: u16, method_idx: u8, func: *mut ObjFunction);
    fn push(&mut self, value: Value);
    fn pop(&mut self) -> Result<Value, String>;
    fn pop_two(&mut self) -> Result<(Value, Value), String>;
    fn peek(&self, distance: usize) -> Result<Value, String>;
    fn current_ip(&self) -> *const u8;
    fn current_ip_mut(&mut self) -> &mut *const u8;
    fn current_locals_mut(&mut self) -> &mut Vec<Value>;
    fn current_locals(&self) -> &Vec<Value>;
    fn get_local(&self, idx: usize) -> Value;
    fn set_local(&mut self, idx: usize, value: Value);
    fn current_chunk(&self) -> &kaubo_ir::Chunk;
    fn get_constant_string(&self, idx: usize) -> String;
    fn current_closure(&self) -> *mut ObjClosure;
    fn current_local_ptr(&mut self, idx: usize) -> *mut Value;
    fn capture_upvalue(&mut self, location: *mut Value) -> *mut ObjUpvalue;
    fn close_upvalues(&mut self, slot: usize);
    fn advance_ip(&mut self, offset: usize);
    fn jump_ip(&mut self, offset: isize);
    fn read_byte(&mut self) -> u8;
    fn read_i16(&mut self) -> i16;
    fn read_u16(&mut self) -> u16;
    fn push_const(&mut self, idx: usize);
    fn stack_top(&self) -> Option<Value>;
    fn shape_count(&self) -> usize;

    // operator/arithmetic delegates
    fn add_values(&self, a: Value, b: Value) -> Result<Value, String>;
    fn sub_values(&self, a: Value, b: Value) -> Result<Value, String>;
    fn mul_values(&self, a: Value, b: Value) -> Result<Value, String>;
    fn div_values(&self, a: Value, b: Value) -> Result<Value, String>;
    fn mod_values(&self, a: Value, b: Value) -> Result<Value, String>;
    fn neg_value(&self, v: Value) -> Result<Value, String>;
    fn compare_values(&self, a: Value, b: Value) -> Result<std::cmp::Ordering, String>;
    fn get_type_name(&self, value: Value) -> &'static str;
    fn find_operator(&self, value: Value, op: Operator) -> Option<*mut ObjClosure>;
    fn get_shape(&self, shape_id: u16) -> *const ObjShape;
    fn get_shape_id(&self, value: Value) -> u16;
    fn call_binary_operator(&mut self, op: Operator, a: Value, b: Value) -> Result<Value, String>;
    fn call_callable_operator(&mut self, op: Operator, args: &[Value]) -> Result<Value, String>;
    fn call_operator_closure_varargs(&mut self, closure: *mut ObjClosure, args: &[Value]) -> Result<Value, String>;
    fn call_operator_closure(&mut self, closure: *mut ObjClosure, args: &[Value]) -> Result<Value, String>;
    fn call_unary_operator(&mut self, op: Operator, value: Value) -> Result<Value, String>;
    fn allocate_inline_cache(&mut self) -> u8;
    fn inline_cache_get(&self, cache_idx: u8, left: Value, right: Value) -> Option<*mut ObjClosure>;
    fn inline_cache_update(&mut self, cache_idx: u8, left: Value, right: Value, closure: *mut ObjClosure);
    fn call_binary_operator_cached(&mut self, op: Operator, a: Value, b: Value, cache_idx: u8) -> Result<Value, String>;
    fn index_get_base(&self, obj_val: Value, index_val: Value) -> Result<Option<Value>, String>;
    fn index_set_base(&mut self, obj_val: Value, key_val: Value, value: Value) -> Result<bool, String>;
    fn call_set_operator(&mut self, obj: Value, index: Value, value: Value) -> Result<(), String>;
}

impl VmRuntime for VM {
    fn init_stdlib(&mut self) {
        use crate::stdlib::create_stdlib_modules;
        let modules = create_stdlib_modules();
        for (name, module) in modules {
            let module_ptr = Box::into_raw(module);
            self.globals.insert(name, Value::module(module_ptr));
        }
    }

    fn stack_mut(&mut self) -> &mut Vec<Value> { &mut self.stack }
    fn frames_mut(&mut self) -> &mut Vec<CallFrame> { &mut self.frames }
    fn open_upvalues_mut(&mut self) -> &mut Vec<*mut ObjUpvalue> { &mut self.open_upvalues }

    fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.interpret_with_locals(chunk, 0)
    }

    fn interpret_with_locals(&mut self, chunk: &Chunk, local_count: usize) -> InterpretResult {
        execution::register_operators_from_chunk(self, chunk);
        self.inline_caches.clear();
        self.inline_caches.extend(chunk.inline_caches.clone());
        // Register shapes first, then methods (methods need shapes to exist)
        for (shape_id, name, field_names, field_types) in &chunk.shape_table {
            let shape = Box::into_raw(Box::new(ObjShape::new_with_types(
                *shape_id, name.clone(), field_names.clone(), field_types.clone(),
            )));
            unsafe { self.register_shape(shape); }
        }
        shape::register_methods_from_chunk(self, chunk);
        let function = Box::into_raw(Box::new(ObjFunction::new(chunk.clone(), 0, Some("<main>".to_string()))));
        let closure = Box::into_raw(Box::new(ObjClosure::new(function)));
        let mut locals = Vec::with_capacity(local_count);
        for _ in 0..local_count { locals.push(Value::NULL); }
        let ip = unsafe { (*(*closure).function).chunk.code.as_ptr() };
        self.frames.push(CallFrame { closure, ip, locals, stack_base: 0 });
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run()));
        let interpret_result = match result {
            Ok(r) => r,
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Internal VM error".to_string()
                };
                if let Some(ref reporter) = self.error_reporter {
                    reporter.report_panic(&msg);
                }
                InterpretResult::runtime_error(format!("VM panic: {}", msg))
            }
        };
        // Report error if reporter is set
        if let InterpretResult::RuntimeError(ref err) = interpret_result {
            if let Some(ref reporter) = self.error_reporter {
                reporter.report_runtime_error(err);
            }
        } else if let InterpretResult::CompileError(ref msg) = interpret_result {
            if let Some(ref reporter) = self.error_reporter {
                reporter.report_compile_error(msg);
            }
        }
        // Try cleanup — ignore errors on already-corrupted state
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.frames.pop();
            call::close_upvalues(self, 0);
        }));
        interpret_result
    }

    fn run(&mut self) -> InterpretResult { execution::run(self) }

    unsafe fn register_shape(&mut self, shape: *const ObjShape) {
        shape::register_shape(self, shape);
    }

    fn register_method_to_shape(&mut self, shape_id: u16, method_idx: u8, func: *mut ObjFunction) {
        shape::register_method_to_shape(self, shape_id, method_idx, func);
    }

    fn push(&mut self, value: Value) { stack::push(self, value); }
    fn pop(&mut self) -> Result<Value, String> { stack::pop(self) }
    fn pop_two(&mut self) -> Result<(Value, Value), String> { stack::pop_two(self) }
    fn peek(&self, distance: usize) -> Result<Value, String> { stack::peek(self, distance) }

    fn current_ip(&self) -> *const u8 { execution::current_ip(self) }
    fn current_ip_mut(&mut self) -> &mut *const u8 { execution::current_ip_mut(self) }
    fn current_locals_mut(&mut self) -> &mut Vec<Value> { execution::current_locals_mut(self) }
    fn current_locals(&self) -> &Vec<Value> { execution::current_locals(self) }
    fn get_local(&self, idx: usize) -> Value { execution::get_local(self, idx) }
    fn set_local(&mut self, idx: usize, value: Value) { execution::set_local(self, idx, value); }
    fn current_chunk(&self) -> &kaubo_ir::Chunk { execution::current_chunk(self) }
    fn get_constant_string(&self, idx: usize) -> String { execution::get_constant_string(self, idx) }
    fn current_closure(&self) -> *mut ObjClosure { execution::current_closure(self) }
    fn advance_ip(&mut self, offset: usize) { execution::advance_ip(self, offset); }
    fn jump_ip(&mut self, offset: isize) { execution::jump_ip(self, offset); }
    fn read_byte(&mut self) -> u8 { execution::read_byte(self) }
    fn read_i16(&mut self) -> i16 { execution::read_i16(self) }
    fn read_u16(&mut self) -> u16 { execution::read_u16(self) }
    fn push_const(&mut self, idx: usize) { execution::push_const(self, idx); }

    fn current_local_ptr(&mut self, idx: usize) -> *mut Value { call::current_local_ptr(self, idx) }
    fn capture_upvalue(&mut self, location: *mut Value) -> *mut ObjUpvalue { call::capture_upvalue(self, location) }
    fn close_upvalues(&mut self, slot: usize) { call::close_upvalues(self, slot); }

    fn stack_top(&self) -> Option<Value> { stack::stack_top(self) }
    fn shape_count(&self) -> usize { self.shapes.len() }

    fn add_values(&self, a: Value, b: Value) -> Result<Value, String> { operators::add_values(self, a, b) }
    fn sub_values(&self, a: Value, b: Value) -> Result<Value, String> { operators::sub_values(self, a, b) }
    fn mul_values(&self, a: Value, b: Value) -> Result<Value, String> { operators::mul_values(self, a, b) }
    fn div_values(&self, a: Value, b: Value) -> Result<Value, String> { operators::div_values(self, a, b) }
    fn mod_values(&self, a: Value, b: Value) -> Result<Value, String> { operators::mod_values(self, a, b) }
    fn neg_value(&self, v: Value) -> Result<Value, String> { operators::neg_value(self, v) }
    fn compare_values(&self, a: Value, b: Value) -> Result<std::cmp::Ordering, String> { operators::compare_values(self, a, b) }
    fn get_type_name(&self, value: Value) -> &'static str { operators::get_type_name(value) }
    fn find_operator(&self, value: Value, op: Operator) -> Option<*mut ObjClosure> { operators::find_operator(self, value, op) }
    fn get_shape(&self, shape_id: u16) -> *const ObjShape { shape::get_shape(self, shape_id) }
    fn get_shape_id(&self, value: Value) -> u16 { operators::get_shape_id(self, value) }
    fn call_binary_operator(&mut self, op: Operator, a: Value, b: Value) -> Result<Value, String> { operators::call_binary_operator(self, op, a, b) }
    fn call_callable_operator(&mut self, op: Operator, args: &[Value]) -> Result<Value, String> { operators::call_callable_operator(self, op, args) }
    fn call_operator_closure_varargs(&mut self, closure: *mut ObjClosure, args: &[Value]) -> Result<Value, String> { operators::call_operator_closure_varargs(self, closure, args) }
    fn call_operator_closure(&mut self, closure: *mut ObjClosure, args: &[Value]) -> Result<Value, String> { operators::call_operator_closure(self, closure, args) }
    fn call_unary_operator(&mut self, op: Operator, value: Value) -> Result<Value, String> { operators::call_unary_operator(self, op, value) }
    fn allocate_inline_cache(&mut self) -> u8 { operators::allocate_inline_cache(self) }
    fn inline_cache_get(&self, cache_idx: u8, left: Value, right: Value) -> Option<*mut ObjClosure> { operators::inline_cache_get(self, cache_idx, left, right) }
    fn inline_cache_update(&mut self, cache_idx: u8, left: Value, right: Value, closure: *mut ObjClosure) { operators::inline_cache_update(self, cache_idx, left, right, closure); }
    fn call_binary_operator_cached(&mut self, op: Operator, a: Value, b: Value, cache_idx: u8) -> Result<Value, String> { operators::call_binary_operator_cached(self, op, a, b, cache_idx) }
    fn index_get_base(&self, obj_val: Value, index_val: Value) -> Result<Option<Value>, String> { index::index_get_base(self, obj_val, index_val) }
    fn index_set_base(&mut self, obj_val: Value, key_val: Value, value: Value) -> Result<bool, String> { index::index_set_base(self, obj_val, key_val, value) }
    fn call_set_operator(&mut self, obj: Value, index: Value, value: Value) -> Result<(), String> { index::call_set_operator(self, obj, index, value) }
}
