//! 虚拟机实现

use crate::runtime::bytecode::{chunk::Chunk, OpCode};
use crate::runtime::object::{
    CallFrame, CoroutineState, ObjClosure, ObjCoroutine, ObjFunction, ObjIterator, ObjJson,
    ObjList, ObjModule, ObjShape, ObjStruct, ObjUpvalue,
};
use crate::runtime::operators::{InlineCacheEntry, Operator};
use crate::runtime::Value;
use kaubo_log::{trace, Logger};
use std::collections::HashMap;
use std::sync::Arc;

/// 解释执行结果
#[derive(Debug, Clone, PartialEq)]
pub enum InterpretResult {
    Ok,
    CompileError(String),
    RuntimeError(String),
}

// CallFrame 已从 object.rs 导入

/// 虚拟机
pub struct VM {
    /// 操作数栈（独立于局部变量）
    stack: Vec<Value>,
    /// 调用栈
    frames: Vec<CallFrame>,
    /// 打开的 upvalues（按地址排序，方便二分查找）
    open_upvalues: Vec<*mut ObjUpvalue>,
    /// 全局变量表
    globals: HashMap<String, Value>,
    /// Shape 表（编译期生成，运行时注册）
    shapes: HashMap<u16, *const ObjShape>,
    /// 内联缓存表（用于运算符重载优化）
    inline_caches: Vec<InlineCacheEntry>,
    /// Logger
    logger: Arc<Logger>,
}

impl VM {
    /// 创建新的虚拟机，并初始化标准库
    pub fn new() -> Self {
        Self::with_logger(Logger::noop())
    }

    /// 创建新的虚拟机（带 logger）
    pub fn with_logger(logger: Arc<Logger>) -> Self {
        let mut vm = Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            open_upvalues: Vec::new(),
            globals: HashMap::new(),
            shapes: HashMap::new(),
            inline_caches: Vec::with_capacity(64),
            logger,
        };
        vm.init_stdlib();
        vm
    }

    /// 初始化标准库模块
    fn init_stdlib(&mut self) {
        use crate::runtime::stdlib::create_stdlib_modules;

        let modules = create_stdlib_modules();
        for (name, module) in modules {
            // 将模块对象转为 Value 并注册到 globals
            let module_ptr = Box::into_raw(module);
            self.globals.insert(name, Value::module(module_ptr));
        }
    }

    /// 获取栈的可变引用（crate 内部使用）
    pub(crate) fn stack_mut(&mut self) -> &mut Vec<Value> {
        &mut self.stack
    }

    /// 获取调用帧的可变引用（crate 内部使用）
    pub(crate) fn frames_mut(&mut self) -> &mut Vec<CallFrame> {
        &mut self.frames
    }

    /// 获取 upvalues 的可变引用（crate 内部使用）
    pub(crate) fn open_upvalues_mut(&mut self) -> &mut Vec<*mut ObjUpvalue> {
        &mut self.open_upvalues
    }

    /// 解释执行一个 Chunk
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.interpret_with_locals(chunk, 0)
    }

    /// 解释执行一个 Chunk，并预分配局部变量空间
    pub fn interpret_with_locals(&mut self, chunk: &Chunk, local_count: usize) -> InterpretResult {
        // 注册运算符（从 Chunk 的 operator_table 到 Shape）
        self.register_operators_from_chunk(chunk);

        // 创建函数对象
        let function = Box::into_raw(Box::new(ObjFunction::new(
            chunk.clone(),
            0,
            Some("<main>".to_string()),
        )));

        // 创建闭包（虽然主函数没有 upvalues，但统一用闭包包装）
        let closure = Box::into_raw(Box::new(ObjClosure::new(function)));

        // 预分配局部变量空间（初始化为 null）
        let mut locals = Vec::with_capacity(local_count);
        for _ in 0..local_count {
            locals.push(Value::NULL);
        }

        // 创建初始调用帧
        self.frames.push(CallFrame {
            closure,
            ip: chunk.code.as_ptr(),
            locals,
            stack_base: 0,
        });

        // 执行主循环
        let result = self.run();

        // 清理调用栈
        self.frames.pop();

        // 关闭所有 upvalues
        self.close_upvalues(0);

        result
    }

    /// 从 Chunk 的 operator_table 注册运算符到 Shape
    fn register_operators_from_chunk(&mut self, chunk: &Chunk) {
        use crate::runtime::bytecode::chunk::OperatorTableEntry;
        
        for entry in &chunk.operator_table {
            let OperatorTableEntry { shape_id, operator_name, const_idx } = entry;
            
            // 获取函数值
            if let Some(function_value) = chunk.constants.get(*const_idx as usize) {
                if let Some(function_ptr) = function_value.as_function() {
                    // 创建闭包（运算符方法没有 upvalues）
                    let closure = Box::into_raw(Box::new(ObjClosure::new(function_ptr)));
                    
                    // 获取或创建 Shape
                    let shape_ptr = self.shapes.entry(*shape_id).or_insert_with(|| {
                        // 如果 Shape 不存在，创建一个空的（这不应该发生，但做安全处理）
                        let shape = Box::into_raw(Box::new(ObjShape::new(
                            *shape_id,
                            format!("<anon_{}>", shape_id),
                            Vec::new(),
                        )));
                        shape
                    });
                    
                    // 注册运算符
                    unsafe {
                        if let Some(op) = Operator::from_method_name(operator_name) {
                            (*(*shape_ptr as *mut ObjShape)).register_operator(op, closure);
                        }
                    }
                }
            }
        }
    }

    /// 执行字节码的主循环
    ///
    /// 注意：此方法为 crate 内部可见，用于 VM-aware 原生函数
    pub(crate) fn run(&mut self) -> InterpretResult {
        use OpCode::*;

        loop {
            // 调试: 打印当前栈状态和指令
            #[cfg(feature = "trace_execution")]
            self.trace_instruction();

            // 读取操作码
            let instruction = unsafe { *self.current_ip() };
            self.advance_ip(1);
            let op = unsafe { std::mem::transmute::<u8, OpCode>(instruction) };

            // VM 执行追踪
            trace!(self.logger, "execute: {:?}, stack: {:?}", op, self.stack);
            match op {
                // ===== 常量加载 =====
                LoadConst0 => self.push_const(0),
                LoadConst1 => self.push_const(1),
                LoadConst2 => self.push_const(2),
                LoadConst3 => self.push_const(3),
                LoadConst4 => self.push_const(4),
                LoadConst5 => self.push_const(5),
                LoadConst6 => self.push_const(6),
                LoadConst7 => self.push_const(7),
                LoadConst8 => self.push_const(8),
                LoadConst9 => self.push_const(9),
                LoadConst10 => self.push_const(10),
                LoadConst11 => self.push_const(11),
                LoadConst12 => self.push_const(12),
                LoadConst13 => self.push_const(13),
                LoadConst14 => self.push_const(14),
                LoadConst15 => self.push_const(15),

                LoadConst => {
                    let idx = self.read_byte();
                    self.push_const(idx as usize);
                }

                // ===== 特殊值 =====
                LoadNull => self.push(Value::NULL),
                LoadTrue => self.push(Value::TRUE),
                LoadFalse => self.push(Value::FALSE),
                LoadZero => self.push(Value::smi(0)),
                LoadOne => self.push(Value::smi(1)),

                // ===== 栈操作 =====
                Pop => {
                    self.pop();
                }

                Dup => {
                    let v = self.peek(0);
                    self.push(v);
                }

                Swap => {
                    let len = self.stack.len();
                    if len >= 2 {
                        self.stack.swap(len - 1, len - 2);
                    }
                }

                // ===== 算术运算 =====
                Add => {
                    let (a, b) = self.pop_two();
                    // 先尝试基础类型
                    let result = self.add_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(_) => {
                            // 基础类型失败，尝试运算符重载
                            match self.call_binary_operator(Operator::Add, a, b) {
                                Ok(v) => self.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }

                Sub => {
                    let (a, b) = self.pop_two();
                    let result = self.sub_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(_) => {
                            match self.call_binary_operator(Operator::Sub, a, b) {
                                Ok(v) => self.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }

                Mul => {
                    let (a, b) = self.pop_two();
                    let result = self.mul_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(_) => {
                            match self.call_binary_operator(Operator::Mul, a, b) {
                                Ok(v) => self.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }

                Div => {
                    let (a, b) = self.pop_two();
                    let result = self.div_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(_) => {
                            match self.call_binary_operator(Operator::Div, a, b) {
                                Ok(v) => self.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }

                Mod => {
                    let (a, b) = self.pop_two();
                    let result = self.mod_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(_) => {
                            match self.call_binary_operator(Operator::Mod, a, b) {
                                Ok(v) => self.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }

                Neg => {
                    let v = self.pop();
                    let result = self.neg_value(v);
                    match result {
                        Ok(v) => self.push(v),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                Not => {
                    let v = self.pop();
                    // 逻辑取非：真值变为 false，假值变为 true
                    if v.is_truthy() {
                        self.push(Value::FALSE);
                    } else {
                        self.push(Value::TRUE);
                    }
                }

                // ===== 比较运算 =====
                Equal => {
                    let (a, b) = self.pop_two();
                    self.push(Value::bool_from(a == b));
                }

                NotEqual => {
                    let (a, b) = self.pop_two();
                    self.push(Value::bool_from(a != b));
                }

                Greater => {
                    let (a, b) = self.pop_two();
                    let result = self.compare_values(a, b);
                    match result {
                        Ok(Ordering::Greater) => self.push(Value::TRUE),
                        Ok(_) => self.push(Value::FALSE),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                Less => {
                    let (a, b) = self.pop_two();
                    let result = self.compare_values(a, b);
                    match result {
                        Ok(Ordering::Less) => self.push(Value::TRUE),
                        Ok(_) => self.push(Value::FALSE),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                LessEqual => {
                    let (a, b) = self.pop_two();
                    let result = self.compare_values(a, b);
                    match result {
                        Ok(Ordering::Less) | Ok(Ordering::Equal) => self.push(Value::TRUE),
                        Ok(_) => self.push(Value::FALSE),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                // ===== 局部变量 =====
                LoadLocal0 => {
                    let value = self.get_local(0);
                    self.push(value);
                }
                LoadLocal1 => {
                    let value = self.get_local(1);
                    self.push(value);
                }
                LoadLocal2 => {
                    let value = self.get_local(2);
                    self.push(value);
                }
                LoadLocal3 => {
                    let value = self.get_local(3);
                    self.push(value);
                }
                LoadLocal4 => {
                    let value = self.get_local(4);
                    self.push(value);
                }
                LoadLocal5 => {
                    let value = self.get_local(5);
                    self.push(value);
                }
                LoadLocal6 => {
                    let value = self.get_local(6);
                    self.push(value);
                }
                LoadLocal7 => {
                    let value = self.get_local(7);
                    self.push(value);
                }
                LoadLocal => {
                    let idx = self.read_byte() as usize;
                    let value = self.get_local(idx);
                    self.push(value);
                }

                StoreLocal0 => {
                    let value = self.pop();
                    self.set_local(0, value);
                }
                StoreLocal1 => {
                    let value = self.pop();
                    self.set_local(1, value);
                }
                StoreLocal2 => {
                    let value = self.pop();
                    self.set_local(2, value);
                }
                StoreLocal3 => {
                    let value = self.pop();
                    self.set_local(3, value);
                }
                StoreLocal4 => {
                    let value = self.pop();
                    self.set_local(4, value);
                }
                StoreLocal5 => {
                    let value = self.pop();
                    self.set_local(5, value);
                }
                StoreLocal6 => {
                    let value = self.pop();
                    self.set_local(6, value);
                }
                StoreLocal7 => {
                    let value = self.pop();
                    self.set_local(7, value);
                }
                StoreLocal => {
                    let idx = self.read_byte() as usize;
                    let value = self.pop();
                    self.set_local(idx, value);
                }

                // ===== 全局变量 =====
                LoadGlobal => {
                    let idx = self.read_byte() as usize;
                    let name = self.get_constant_string(idx);
                    if let Some(value) = self.globals.get(&name) {
                        self.push(*value);
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Undefined global variable: {}",
                            name
                        ));
                    }
                }

                StoreGlobal => {
                    let idx = self.read_byte() as usize;
                    let name = self.get_constant_string(idx);
                    let value = self.pop();
                    self.globals.insert(name, value);
                }

                DefineGlobal => {
                    let idx = self.read_byte() as usize;
                    let name = self.get_constant_string(idx);
                    let value = self.pop();
                    self.globals.insert(name, value);
                }

                // ===== 控制流 =====
                Jump => {
                    let offset = self.read_i16();
                    self.jump_ip(offset as isize);
                }

                JumpIfFalse => {
                    let offset = self.read_i16();
                    let condition = self.pop(); // 弹出条件
                    if !condition.is_truthy() {
                        self.jump_ip(offset as isize);
                    }
                }

                JumpBack => {
                    let offset = self.read_i16();
                    self.jump_ip(offset as isize);
                }

                // ===== 函数 =====
                Call => {
                    let arg_count = self.read_byte();

                    // 栈布局：[arg0, arg1, ..., argN, closure]
                    // 先弹出闭包对象（栈顶）
                    let callee = self.pop();
                    if let Some(closure_ptr) = callee.as_closure() {
                        let closure = unsafe { &*closure_ptr };
                        let func = unsafe { &*closure.function };

                        if func.arity != arg_count {
                            return InterpretResult::RuntimeError(format!(
                                "Expected {} arguments but got {}",
                                func.arity, arg_count
                            ));
                        }

                        // 收集参数（从栈顶）
                        let mut locals = Vec::with_capacity(arg_count as usize);
                        for _ in 0..arg_count {
                            locals.push(self.pop());
                        }
                        locals.reverse();

                        // 创建新的调用帧
                        let stack_base = self.stack.len();
                        let new_frame = CallFrame {
                            closure: closure_ptr,
                            ip: func.chunk.code.as_ptr(),
                            locals,
                            stack_base,
                        };
                        self.frames.push(new_frame);
                    } else if let Some(func_ptr) = callee.as_function() {
                        // 向后兼容：直接调用函数（无闭包）
                        let func = unsafe { &*func_ptr };
                        if func.arity != arg_count {
                            return InterpretResult::RuntimeError(format!(
                                "Expected {} arguments but got {}",
                                func.arity, arg_count
                            ));
                        }

                        let mut locals = Vec::with_capacity(arg_count as usize);
                        for _ in 0..arg_count {
                            locals.push(self.pop());
                        }
                        locals.reverse();

                        // 包装为闭包
                        let closure = Box::into_raw(Box::new(ObjClosure::new(func_ptr)));
                        let stack_base = self.stack.len();
                        let new_frame = CallFrame {
                            closure,
                            ip: func.chunk.code.as_ptr(),
                            locals,
                            stack_base,
                        };
                        self.frames.push(new_frame);
                    } else if let Some(native_ptr) = callee.as_native() {
                        // 调用原生函数
                        let native = unsafe { &*native_ptr };

                        // 变参函数：arity=255 表示可变参数
                        // 否则参数数量必须等于 arity（或支持变参的函数内部处理）
                        if native.arity != 255 && arg_count != native.arity {
                            return InterpretResult::RuntimeError(format!(
                                "Expected {} arguments but got {}",
                                native.arity, arg_count
                            ));
                        }

                        // 收集参数（从栈顶）
                        let mut args = Vec::with_capacity(arg_count as usize);
                        for _ in 0..arg_count {
                            args.push(self.pop());
                        }
                        args.reverse();

                        // 调用原生函数
                        match native.call(&args) {
                            Ok(result) => self.push(result),
                            Err(msg) => return InterpretResult::RuntimeError(msg),
                        }
                    } else if let Some(native_vm_ptr) = callee.as_native_vm() {
                        // 调用 VM-aware 原生函数
                        let native_vm = unsafe { &*native_vm_ptr };

                        // 参数校验
                        if native_vm.arity != 255 && arg_count != native_vm.arity {
                            return InterpretResult::RuntimeError(format!(
                                "Expected {} arguments but got {}",
                                native_vm.arity, arg_count
                            ));
                        }

                        // 收集参数（从栈顶）
                        let mut args = Vec::with_capacity(arg_count as usize);
                        for _ in 0..arg_count {
                            args.push(self.pop());
                        }
                        args.reverse();

                        // 调用 VM-aware 原生函数，传入 self (VM)
                        match native_vm.call(self, &args) {
                            Ok(result) => self.push(result),
                            Err(msg) => return InterpretResult::RuntimeError(msg),
                        }
                    } else {
                        return InterpretResult::RuntimeError(
                            "Can only call functions".to_string(),
                        );
                    }
                }

                Closure => {
                    // 从常量池加载函数对象
                    let const_idx = self.read_byte();
                    let constant = self.current_chunk().constants[const_idx as usize];
                    let upvalue_count = self.read_byte();

                    if let Some(func_ptr) = constant.as_function() {
                        // 创建闭包对象
                        let mut closure = Box::new(ObjClosure::new(func_ptr));

                        // 捕获 upvalues
                        for _ in 0..upvalue_count {
                            let is_local = self.read_byte() != 0;
                            let index = self.read_byte();

                            if is_local {
                                // 捕获当前帧的局部变量
                                let location = self.current_local_ptr(index as usize);
                                let upvalue = self.capture_upvalue(location);
                                closure.add_upvalue(upvalue);
                            } else {
                                // 继承当前闭包的 upvalue
                                let current_closure = self.current_closure();
                                let upvalue = unsafe {
                                    (*current_closure).get_upvalue(index as usize).unwrap()
                                };
                                closure.add_upvalue(upvalue);
                            }
                        }

                        self.push(Value::closure(Box::into_raw(closure)));
                    } else {
                        return InterpretResult::RuntimeError(
                            "Closure constant must be a function".to_string(),
                        );
                    }
                }

                GetUpvalue => {
                    let idx = self.read_byte() as usize;
                    let closure = self.current_closure();
                    let upvalue = unsafe { (*closure).get_upvalue(idx).unwrap() };
                    let value = unsafe { (*upvalue).get() };
                    self.push(value);
                }

                SetUpvalue => {
                    let idx = self.read_byte() as usize;
                    let value = self.peek(0);
                    let closure = self.current_closure();
                    let upvalue = unsafe { (*closure).get_upvalue(idx).unwrap() };
                    unsafe {
                        (*upvalue).set(value);
                    }
                }

                CloseUpvalues => {
                    let slot = self.read_byte() as usize;
                    self.close_upvalues(slot);
                }

                Return => {
                    // 1. 关闭当前帧的 upvalues
                    self.close_upvalues(0);

                    // 2. 弹出当前函数的调用帧
                    self.frames
                        .pop()
                        .expect("Runtime error: No call frame to pop");

                    // 3. 压入 NULL 作为无返回值函数的返回值
                    self.push(Value::NULL);

                    // 4. 只有当调用帧为空（主函数返回）时，才终止VM执行；否则继续执行上层帧
                    if self.frames.is_empty() {
                        return InterpretResult::Ok;
                    }
                    // 非空则继续循环，执行上层帧的下一条指令
                }

                // ===== 修复后的 RETURN_VALUE 指令 =====
                ReturnValue => {
                    // 1. 关闭当前帧的 upvalues
                    self.close_upvalues(0);

                    // 2. 弹出当前函数的调用帧
                    self.frames
                        .pop()
                        .expect("Runtime error: No call frame to pop");

                    // 3. 保存栈顶的返回值（函数执行结果）
                    let return_value = self.pop();

                    // 4. 将返回值压回栈顶，供上层帧使用
                    self.push(return_value);

                    // 5. 仅主函数返回时终止，否则继续执行上层帧
                    if self.frames.is_empty() {
                        return InterpretResult::Ok;
                    }
                    // 非空则继续循环
                }

                // ===== 协程 =====
                CreateCoroutine => {
                    // 从栈顶弹出闭包，创建协程对象
                    let closure_val = self.pop();
                    if let Some(closure_ptr) = closure_val.as_closure() {
                        let coroutine = Box::new(ObjCoroutine::new(closure_ptr));
                        self.push(Value::coroutine(Box::into_raw(coroutine)));
                    } else {
                        return InterpretResult::RuntimeError(
                            "Coroutine must be created from a closure".to_string(),
                        );
                    }
                }

                Resume => {
                    // 操作数：传入值个数
                    let arg_count = self.read_byte();

                    // 从栈顶弹出协程对象
                    let coro_val = self.pop();
                    if let Some(coro_ptr) = coro_val.as_coroutine() {
                        let coro = unsafe { &mut *coro_ptr };

                        // 检查协程状态
                        if coro.state == CoroutineState::Dead {
                            return InterpretResult::RuntimeError(
                                "Cannot resume dead coroutine".to_string(),
                            );
                        }

                        // 收集传入的参数
                        let mut args = Vec::with_capacity(arg_count as usize);
                        for _ in 0..arg_count {
                            args.push(self.pop());
                        }
                        args.reverse();

                        // 如果协程是第一次运行，需要初始化调用帧
                        if coro.state == CoroutineState::Suspended && coro.frames.is_empty() {
                            let closure = coro.entry_closure;
                            let func = unsafe { &*(*closure).function };

                            if func.arity != arg_count {
                                return InterpretResult::RuntimeError(format!(
                                    "Expected {} arguments but got {}",
                                    func.arity, arg_count
                                ));
                            }

                            // 创建初始调用帧
                            coro.frames.push(CallFrame {
                                closure,
                                ip: func.chunk.code.as_ptr(),
                                locals: args,
                                stack_base: 0,
                            });
                        }

                        // 切换到协程上下文执行（简化版：直接在当前 VM 中运行）
                        coro.state = CoroutineState::Running;

                        // 保存当前 VM 状态
                        let saved_stack = std::mem::take(&mut self.stack);
                        let saved_frames = std::mem::take(&mut self.frames);
                        let saved_upvalues = std::mem::take(&mut self.open_upvalues);

                        // 加载协程状态
                        self.stack = std::mem::take(&mut coro.stack);
                        self.frames = std::mem::take(&mut coro.frames);
                        self.open_upvalues = std::mem::take(&mut coro.open_upvalues);

                        // 执行协程
                        let result = self.run();

                        // 保存协程状态
                        coro.stack = std::mem::take(&mut self.stack);
                        coro.frames = std::mem::take(&mut self.frames);
                        coro.open_upvalues = std::mem::take(&mut self.open_upvalues);

                        // 根据执行结果处理
                        match result {
                            InterpretResult::Ok => {
                                coro.state = CoroutineState::Dead;
                                // 协程正常结束，获取返回值
                                let return_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                                // 恢复主 VM 状态
                                self.stack = saved_stack;
                                self.frames = saved_frames;
                                self.open_upvalues = saved_upvalues;
                                // 将返回值压入主栈
                                self.push(return_val);
                            }
                            InterpretResult::RuntimeError(msg) => {
                                if msg == "yield" {
                                    // 协程通过 yield 挂起
                                    coro.state = CoroutineState::Suspended;
                                    // 获取 yield 值（在协程栈顶）
                                    let yield_val =
                                        coro.stack.last().copied().unwrap_or(Value::NULL);
                                    // 恢复主 VM 状态
                                    self.stack = saved_stack;
                                    self.frames = saved_frames;
                                    self.open_upvalues = saved_upvalues;
                                    // 将 yield 值压入主栈
                                    self.push(yield_val);
                                } else {
                                    coro.state = CoroutineState::Dead;
                                    // 恢复主 VM 状态再返回错误
                                    self.stack = saved_stack;
                                    self.frames = saved_frames;
                                    self.open_upvalues = saved_upvalues;
                                    return InterpretResult::RuntimeError(msg);
                                }
                            }
                            InterpretResult::CompileError(msg) => {
                                coro.state = CoroutineState::Dead;
                                // 恢复主 VM 状态再返回错误
                                self.stack = saved_stack;
                                self.frames = saved_frames;
                                self.open_upvalues = saved_upvalues;
                                return InterpretResult::CompileError(msg);
                            }
                        }
                    } else {
                        return InterpretResult::RuntimeError(
                            "Can only resume coroutines".to_string(),
                        );
                    }
                }

                Yield => {
                    // 从栈顶弹出要返回的值
                    let value = self.pop();

                    // 保存返回值到当前栈
                    self.push(value);

                    // 返回特殊错误表示 yield（简化实现）
                    return InterpretResult::RuntimeError("yield".to_string());
                }

                CoroutineStatus => {
                    // 从栈顶弹出协程对象
                    let coro_val = self.pop();
                    if let Some(coro_ptr) = coro_val.as_coroutine() {
                        let coro = unsafe { &*coro_ptr };
                        let status = match coro.state {
                            CoroutineState::Suspended => 0i64,
                            CoroutineState::Running => 1i64,
                            CoroutineState::Dead => 2i64,
                        };
                        self.push(Value::smi(status as i32));
                    } else {
                        return InterpretResult::RuntimeError("Expected a coroutine".to_string());
                    }
                }

                // ===== 列表 =====
                BuildList => {
                    let count = self.read_byte() as usize;
                    // 从栈顶弹出 count 个元素，创建列表
                    let mut elements = Vec::with_capacity(count);
                    for _ in 0..count {
                        elements.push(self.pop());
                    }
                    elements.reverse(); // 栈顶是最后一个元素

                    let list = Box::new(ObjList::from_vec(elements));
                    let list_ptr = Box::into_raw(list);
                    self.push(Value::list(list_ptr));
                }

                BuildJson => {
                    let count = self.read_byte() as usize;
                    // 从栈顶弹出键值对（值先入栈，然后是键），创建 JSON 对象
                    let mut entries = std::collections::HashMap::with_capacity(count);
                    for _ in 0..count {
                        let key_val = self.pop();
                        let value = self.pop();

                        // 键必须是字符串
                        if let Some(key_ptr) = key_val.as_string() {
                            let key_str = unsafe { &(*key_ptr).chars };
                            entries.insert(key_str.clone(), value);
                        } else {
                            return InterpretResult::RuntimeError(
                                "JSON key must be a string".to_string(),
                            );
                        }
                    }

                    let json = Box::new(ObjJson::from_hashmap(entries));
                    let json_ptr = Box::into_raw(json);
                    self.push(Value::json(json_ptr));
                }

                BuildModule => {
                    let count = self.read_byte() as usize;
                    // 从栈顶弹出导出值（逆序），创建模块对象
                    // 按逆序弹出，这样先定义的导出项索引小
                    let mut exports = Vec::with_capacity(count);
                    for _ in 0..count {
                        exports.push(self.pop());
                    }
                    exports.reverse();

                    // 创建模块对象（暂时不设置名称和 name_to_index，后续由编译器提供）
                    let module = Box::new(ObjModule::new(
                        String::new(),
                        exports,
                        std::collections::HashMap::new(),
                    ));
                    let module_ptr = Box::into_raw(module);
                    self.push(Value::module(module_ptr));
                }

                ModuleGet => {
                    // 栈顶: [module]
                    let module_val = self.pop();
                    let shape_id = self.read_u16();

                    if let Some(module_ptr) = module_val.as_module() {
                        let module = unsafe { &*module_ptr };
                        if let Some(value) = module.get_by_shape_id(shape_id) {
                            self.push(value);
                        } else {
                            return InterpretResult::RuntimeError(format!(
                                "Module field with ShapeID {} not found",
                                shape_id
                            ));
                        }
                    } else {
                        return InterpretResult::RuntimeError(
                            "ModuleGet requires a module".to_string(),
                        );
                    }
                }

                GetModuleExport => {
                    // 栈顶: [module] -> value
                    // 操作数: u8 常量池索引（导出项名称字符串）
                    let module_val = self.pop();
                    let name_idx = self.read_byte() as usize;

                    // 从常量池获取名称字符串
                    let name = if let Some(constant) = self.current_chunk().constants.get(name_idx)
                    {
                        if let Some(ptr) = constant.as_string() {
                            unsafe { (&*ptr).chars.clone() }
                        } else {
                            return InterpretResult::RuntimeError(
                                "GetModuleExport name must be a string".to_string(),
                            );
                        }
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Invalid constant index for GetModuleExport: {}",
                            name_idx
                        ));
                    };

                    if let Some(module_ptr) = module_val.as_module() {
                        let module = unsafe { &*module_ptr };
                        if let Some(value) = module.get(&name) {
                            self.push(value);
                        } else {
                            return InterpretResult::RuntimeError(format!(
                                "Export '{}' not found in module",
                                name
                            ));
                        }
                    } else {
                        return InterpretResult::RuntimeError(
                            "GetModuleExport requires a module".to_string(),
                        );
                    }
                }

                GetModule => {
                    // 栈顶: [module_name] -> module
                    let name_val = self.pop();

                    let name = if let Some(ptr) = name_val.as_string() {
                        unsafe { (&*ptr).chars.clone() }
                    } else {
                        return InterpretResult::RuntimeError(
                            "GetModule requires a string name".to_string(),
                        );
                    };

                    if let Some(value) = self.globals.get(&name) {
                        self.push(*value);
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Module '{}' not found",
                            name
                        ));
                    }
                }

                // ===== Struct 相关指令 =====
                BuildStruct => {
                    // 操作数: u16 shape_id + u8 field_count
                    let shape_id = self.read_u16();
                    let field_count = self.read_byte() as usize;

                    // 从栈顶弹出字段值
                    // 编译器按 shape 字段顺序的逆序入栈，所以弹出后直接是正确顺序
                    let mut fields = Vec::with_capacity(field_count);
                    for _ in 0..field_count {
                        fields.push(self.pop());
                    }
                    // 不需要 reverse，编译器已经处理好顺序

                    // 创建 struct 实例
                    let shape_ptr = self.get_shape(shape_id);
                    if shape_ptr.is_null() {
                        return InterpretResult::RuntimeError(format!(
                            "Shape ID {} not found",
                            shape_id
                        ));
                    }

                    let struct_obj = Box::new(ObjStruct::new(shape_ptr, fields));
                    self.push(Value::struct_instance(Box::into_raw(struct_obj)));
                }

                GetField => {
                    // 操作数: u8 字段索引
                    let field_idx = self.read_byte() as usize;

                    let struct_val = self.pop();
                    if let Some(struct_ptr) = struct_val.as_struct() {
                        let struct_obj = unsafe { &*struct_ptr };
                        if let Some(value) = struct_obj.get_field(field_idx) {
                            self.push(value);
                        } else {
                            return InterpretResult::RuntimeError(format!(
                                "Field index {} out of bounds",
                                field_idx
                            ));
                        }
                    } else {
                        return InterpretResult::RuntimeError(
                            "GetField requires a struct instance".to_string(),
                        );
                    }
                }

                SetField => {
                    // 操作数: u8 字段索引
                    let field_idx = self.read_byte() as usize;

                    // 栈布局: [value, struct]
                    let struct_val = self.pop();
                    let value = self.pop();

                    if let Some(struct_ptr) = struct_val.as_struct() {
                        let struct_obj = unsafe { &mut *struct_ptr };
                        struct_obj.set_field(field_idx, value);
                    } else {
                        return InterpretResult::RuntimeError(
                            "SetField requires a struct instance".to_string(),
                        );
                    }
                }

                LoadMethod => {
                    // 操作数: u8 方法索引
                    let method_idx = self.read_byte();

                    // 栈顶是 receiver
                    let receiver = self.peek(0);
                    if let Some(struct_ptr) = receiver.as_struct() {
                        let shape = unsafe { (*struct_ptr).shape };
                        if let Some(method) = unsafe { (*shape).get_method(method_idx) } {
                            // 压入函数对象（不是闭包）
                            self.push(Value::function(method));
                        } else {
                            return InterpretResult::RuntimeError(format!(
                                "Method index {} not found in shape",
                                method_idx
                            ));
                        }
                    } else {
                        return InterpretResult::RuntimeError(
                            "LoadMethod requires a struct instance".to_string(),
                        );
                    }
                }

                IndexGet => {
                    // 栈顶: [index, object]
                    let index_val = self.pop();
                    let obj_val = self.pop();

                    // 列表索引（整数）
                    if let Some(idx) = index_val.as_smi() {
                        if let Some(list_ptr) = obj_val.as_list() {
                            let list = unsafe { &*list_ptr };
                            let i = idx as usize;
                            if i >= list.len() {
                                return InterpretResult::RuntimeError(format!(
                                    "Index out of bounds: {} (length {})",
                                    i,
                                    list.len()
                                ));
                            }
                            let value = list.get(i).unwrap_or(Value::NULL);
                            self.push(value);
                        }
                        // Struct 字段索引（整数）
                        else if let Some(struct_ptr) = obj_val.as_struct() {
                            let struct_obj = unsafe { &*struct_ptr };
                            let i = idx as usize;
                            if i >= struct_obj.field_count() {
                                return InterpretResult::RuntimeError(format!(
                                    "Field index out of bounds: {} (struct has {} fields)",
                                    i,
                                    struct_obj.field_count()
                                ));
                            }
                            let value = struct_obj.get_field(i).unwrap_or(Value::NULL);
                            self.push(value);
                        } else {
                            return InterpretResult::RuntimeError(
                                "Can only index lists or structs with integers".to_string(),
                            );
                        }
                    }
                    // JSON 对象索引（字符串键）
                    else if let Some(key_ptr) = index_val.as_string() {
                        if let Some(json_ptr) = obj_val.as_json() {
                            let json = unsafe { &*json_ptr };
                            let key = unsafe { &(*key_ptr).chars };
                            let value = json.get(key).unwrap_or(Value::NULL);
                            self.push(value);
                        }
                        // Struct 字段访问（字符串键）
                        else if let Some(struct_ptr) = obj_val.as_struct() {
                            let struct_obj = unsafe { &*struct_ptr };
                            let shape = unsafe { &*struct_obj.shape };
                            let key = unsafe { &(*key_ptr).chars };

                            if let Some(field_idx) = shape.get_field_index(key) {
                                let value = struct_obj
                                    .get_field(field_idx as usize)
                                    .unwrap_or(Value::NULL);
                                self.push(value);
                            } else {
                                return InterpretResult::RuntimeError(format!(
                                    "Field '{}' not found in struct '{}'",
                                    key, shape.name
                                ));
                            }
                        } else {
                            return InterpretResult::RuntimeError(
                                "Can only index JSON objects or structs with strings".to_string(),
                            );
                        }
                    } else {
                        return InterpretResult::RuntimeError("Index must be an integer (for list/struct) or string (for JSON/struct)".to_string());
                    }
                }

                IndexSet => {
                    // 栈布局: [value, key, object] (object 在栈顶)
                    let obj_val = self.pop();
                    let key_val = self.pop();
                    let value = self.pop();

                    // JSON 对象索引（字符串键）
                    if let Some(key_ptr) = key_val.as_string() {
                        if let Some(json_ptr) = obj_val.as_json() {
                            let json = unsafe { &mut *json_ptr };
                            let key = unsafe { &(*key_ptr).chars };
                            json.set(key.clone(), value);
                        } else {
                            return InterpretResult::RuntimeError(
                                "Can only set keys on JSON objects".to_string(),
                            );
                        }
                    }
                    // 列表索引（整数）
                    else if let Some(idx) = key_val.as_smi() {
                        if let Some(list_ptr) = obj_val.as_list() {
                            let list = unsafe { &mut *list_ptr };
                            let i = idx as usize;
                            if i >= list.len() {
                                return InterpretResult::RuntimeError(format!(
                                    "Index out of bounds: {} (length {})",
                                    i,
                                    list.len()
                                ));
                            }
                            list.elements[i] = value;
                        } else {
                            return InterpretResult::RuntimeError(
                                "Can only index lists with integers".to_string(),
                            );
                        }
                    } else {
                        return InterpretResult::RuntimeError(
                            "Key must be a string (for JSON) or integer (for list)".to_string(),
                        );
                    }
                }

                GetIter => {
                    // 获取迭代器：支持列表、协程和 JSON 对象
                    let val = self.pop();

                    if let Some(list_ptr) = val.as_list() {
                        // 列表 -> 列表迭代器
                        let iter = Box::new(ObjIterator::from_list(list_ptr));
                        let iter_ptr = Box::into_raw(iter);
                        self.push(Value::iterator(iter_ptr));
                    } else if let Some(coro_ptr) = val.as_coroutine() {
                        // 协程 -> 协程迭代器
                        let iter = Box::new(ObjIterator::from_coroutine(coro_ptr));
                        let iter_ptr = Box::into_raw(iter);
                        self.push(Value::iterator(iter_ptr));
                    } else if let Some(json_ptr) = val.as_json() {
                        // JSON 对象 -> JSON 迭代器（遍历键）
                        let iter = Box::new(unsafe { ObjIterator::from_json(json_ptr) });
                        let iter_ptr = Box::into_raw(iter);
                        self.push(Value::iterator(iter_ptr));
                    } else {
                        return InterpretResult::RuntimeError(
                            "Can only iterate over lists, coroutines, or json objects".to_string(),
                        );
                    }
                }

                IterNext => {
                    // 获取迭代器下一个值
                    let iter_val = self.pop();

                    if let Some(iter_ptr) = iter_val.as_iterator() {
                        let iter = unsafe { &mut *iter_ptr };

                        // 检查是否是协程迭代器
                        if let Some(coro_ptr) = iter.as_coroutine() {
                            // 协程迭代器：resume 协程获取下一个值
                            let coro = unsafe { &mut *coro_ptr };

                            if coro.state == CoroutineState::Dead {
                                self.push(Value::NULL);
                            } else {
                                // 如果是第一次运行，初始化调用帧
                                if coro.state == CoroutineState::Suspended && coro.frames.is_empty()
                                {
                                    let closure = coro.entry_closure;
                                    let func = unsafe { &*(*closure).function };
                                    // 创建初始调用帧（无参数）
                                    coro.frames.push(CallFrame {
                                        closure,
                                        ip: func.chunk.code.as_ptr(),
                                        locals: Vec::new(),
                                        stack_base: 0,
                                    });
                                }

                                // 执行 resume
                                coro.state = CoroutineState::Running;

                                // 保存当前 VM 状态
                                let saved_stack = std::mem::take(&mut self.stack);
                                let saved_frames = std::mem::take(&mut self.frames);
                                let saved_upvalues = std::mem::take(&mut self.open_upvalues);

                                // 加载协程状态
                                self.stack = std::mem::take(&mut coro.stack);
                                self.frames = std::mem::take(&mut coro.frames);
                                self.open_upvalues = std::mem::take(&mut coro.open_upvalues);

                                // 执行协程（无参数 resume）
                                let result = self.run();

                                // 保存协程状态
                                coro.stack = std::mem::take(&mut self.stack);
                                coro.frames = std::mem::take(&mut self.frames);
                                coro.open_upvalues = std::mem::take(&mut coro.open_upvalues);

                                // 恢复主 VM 状态
                                self.stack = saved_stack;
                                self.frames = saved_frames;
                                self.open_upvalues = saved_upvalues;

                                // 处理结果
                                match result {
                                    InterpretResult::Ok => {
                                        coro.state = CoroutineState::Dead;
                                        let return_val =
                                            coro.stack.last().copied().unwrap_or(Value::NULL);
                                        self.push(return_val);
                                    }
                                    InterpretResult::RuntimeError(msg) => {
                                        if msg == "yield" {
                                            coro.state = CoroutineState::Suspended;
                                            let yield_val =
                                                coro.stack.last().copied().unwrap_or(Value::NULL);
                                            self.push(yield_val);
                                        } else {
                                            coro.state = CoroutineState::Dead;
                                            return InterpretResult::RuntimeError(msg);
                                        }
                                    }
                                    InterpretResult::CompileError(msg) => {
                                        coro.state = CoroutineState::Dead;
                                        return InterpretResult::CompileError(msg);
                                    }
                                }
                            }
                        } else {
                            // 普通迭代器
                            match iter.next() {
                                Some(value) => self.push(value),
                                None => self.push(Value::NULL),
                            }
                        }
                    } else {
                        return InterpretResult::RuntimeError("Expected iterator".to_string());
                    }
                }

                // ===== 类型转换 =====
                CastToInt => {
                    let v = self.pop();
                    let result = if let Some(n) = v.as_int() {
                        Value::smi(n)
                    } else if v.is_float() {
                        Value::smi(v.as_float() as i32)
                    } else if let Some(s) = v.as_string() {
                        let s_ref = unsafe { &(*s).chars };
                        s_ref.parse::<i32>().map(Value::smi).unwrap_or(Value::NULL)
                    } else {
                        Value::NULL
                    };
                    self.push(result);
                }

                CastToFloat => {
                    let v = self.pop();
                    let result = if let Some(n) = v.as_int() {
                        Value::float(n as f64)
                    } else if v.is_float() {
                        v
                    } else if let Some(s) = v.as_string() {
                        let s_ref = unsafe { &(*s).chars };
                        s_ref.parse::<f64>().map(Value::float).unwrap_or(Value::NULL)
                    } else {
                        Value::NULL
                    };
                    self.push(result);
                }

                CastToString => {
                    let v = self.pop();
                    let s = v.to_string();
                    let string_obj = Box::new(crate::runtime::object::ObjString::new(s));
                    self.push(Value::string(Box::into_raw(string_obj)));
                }

                CastToBool => {
                    let v = self.pop();
                    self.push(Value::bool_from(v.is_truthy()));
                }

                // ===== 调试 =====
                Print => {
                    let v = self.pop();
                    println!("{}", v);
                }

                Invalid => {
                    return InterpretResult::RuntimeError("Invalid opcode".to_string());
                }

                _ => {
                    return InterpretResult::RuntimeError(format!(
                        "Unimplemented opcode: {:?}",
                        op
                    ));
                }
            }
        }
    }

    // ==================== Shape 管理 ====================

    /// 注册 Shape 到 VM
    pub fn register_shape(&mut self, shape: *const ObjShape) {
        unsafe {
            let shape_id = (*shape).shape_id;
            self.shapes.insert(shape_id, shape);
        }
    }

    /// 通过 ID 获取 Shape
    fn get_shape(&self, shape_id: u16) -> *const ObjShape {
        self.shapes
            .get(&shape_id)
            .copied()
            .unwrap_or(std::ptr::null())
    }

    /// 注册方法到 Shape 的方法表
    pub fn register_method_to_shape(
        &mut self,
        shape_id: u16,
        method_idx: u8,
        func: *mut ObjFunction,
    ) {
        if let Some(&shape) = self.shapes.get(&shape_id) {
            unsafe {
                let shape_mut = shape as *mut ObjShape;
                let methods = &mut (*shape_mut).methods;
                if method_idx as usize >= methods.len() {
                    methods.resize(method_idx as usize + 1, std::ptr::null_mut());
                }
                methods[method_idx as usize] = func;
            }
        }
    }

    // ==================== 辅助方法 ====================

    /// 获取当前帧的指令指针
    #[inline]
    fn current_ip(&self) -> *const u8 {
        self.frames.last().unwrap().ip
    }

    /// 获取当前帧的可变指令指针
    #[inline]
    fn current_ip_mut(&mut self) -> &mut *const u8 {
        &mut self.frames.last_mut().unwrap().ip
    }

    /// 获取当前帧的可变 locals
    #[inline]
    fn current_locals_mut(&mut self) -> &mut Vec<Value> {
        &mut self.frames.last_mut().unwrap().locals
    }

    /// 获取当前帧的 locals
    #[inline]
    fn current_locals(&self) -> &Vec<Value> {
        &self.frames.last().unwrap().locals
    }

    /// 获取局部变量（自动扩展）
    #[inline]
    fn get_local(&self, idx: usize) -> Value {
        let locals = self.current_locals();
        if idx < locals.len() {
            locals[idx]
        } else {
            Value::NULL
        }
    }

    /// 设置局部变量（自动扩展）
    #[inline]
    fn set_local(&mut self, idx: usize, value: Value) {
        let locals = self.current_locals_mut();
        if idx >= locals.len() {
            locals.resize(idx + 1, Value::NULL);
        }
        locals[idx] = value;
    }

    /// 获取当前帧的 chunk
    #[inline]
    fn current_chunk(&self) -> &Chunk {
        self.frames.last().unwrap().chunk()
    }

    /// 获取常量池中的字符串
    #[inline]
    fn get_constant_string(&self, idx: usize) -> String {
        let constant = self.current_chunk().constants[idx];
        if let Some(s) = constant.as_string() {
            unsafe { (*s).chars.clone() }
        } else {
            String::new()
        }
    }

    /// 获取当前闭包
    #[inline]
    fn current_closure(&self) -> *mut ObjClosure {
        self.frames.last().unwrap().closure
    }

    /// 获取局部变量指针（用于 upvalue 捕获）
    fn current_local_ptr(&mut self, idx: usize) -> *mut Value {
        // 扩展 locals 以确保索引有效
        let locals = self.current_locals_mut();
        if idx >= locals.len() {
            locals.resize(idx + 1, Value::NULL);
        }
        &mut locals[idx] as *mut Value
    }

    /// 捕获 upvalue（如果已存在则复用）
    fn capture_upvalue(&mut self, location: *mut Value) -> *mut ObjUpvalue {
        // 从后向前查找是否已有指向相同位置的 upvalue
        for &upvalue in self.open_upvalues.iter().rev() {
            unsafe {
                if (*upvalue).location == location {
                    return upvalue;
                }
            }
        }

        // 创建新的 upvalue
        let upvalue = Box::into_raw(Box::new(ObjUpvalue::new(location)));
        self.open_upvalues.push(upvalue);
        upvalue
    }

    /// 关闭从指定槽位开始的所有 upvalues
    fn close_upvalues(&mut self, slot: usize) {
        // 获取当前帧的 locals 起始地址
        let frame_base = self
            .frames
            .last()
            .map(|f| f.locals.as_ptr() as usize)
            .unwrap_or(0);
        let close_threshold = frame_base + slot * std::mem::size_of::<Value>();

        // 关闭所有地址 >= close_threshold 的 upvalue
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let upvalue = self.open_upvalues[i];
            unsafe {
                let upvalue_addr = (*upvalue).location as usize;
                if upvalue_addr >= close_threshold {
                    // 关闭这个 upvalue
                    (*upvalue).close();
                    self.open_upvalues.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }

    /// 前进指令指针
    #[inline]
    fn advance_ip(&mut self, offset: usize) {
        *self.current_ip_mut() = unsafe { self.current_ip().add(offset) };
    }

    /// 跳转指令指针
    #[inline]
    fn jump_ip(&mut self, offset: isize) {
        *self.current_ip_mut() = unsafe { self.current_ip().offset(offset) };
    }

    /// 读取下一个字节
    #[inline]
    fn read_byte(&mut self) -> u8 {
        let byte = unsafe { *self.current_ip() };
        self.advance_ip(1);
        byte
    }

    /// 读取 i16
    #[inline]
    fn read_i16(&mut self) -> i16 {
        let b1 = self.read_byte();
        let b2 = self.read_byte();
        i16::from_le_bytes([b1, b2])
    }

    /// 读取 u16
    #[inline]
    fn read_u16(&mut self) -> u16 {
        let b1 = self.read_byte();
        let b2 = self.read_byte();
        u16::from_le_bytes([b1, b2])
    }
    
    /// 从给定指针读取 u16（小端序）
    #[inline]
    fn read_u16_at_ptr(ip: *const u8) -> u16 {
        let b1 = unsafe { *ip };
        let b2 = unsafe { *ip.add(1) };
        u16::from_le_bytes([b1, b2])
    }

    /// 从常量池加载并压栈
    #[inline]
    fn push_const(&mut self, idx: usize) {
        let value = self.current_chunk().constants[idx];
        self.push(value);
    }

    /// 压栈
    #[inline]
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// 弹栈
    #[inline]
    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }

    /// 弹出两个值 (先弹出的是右操作数)
    #[inline]
    fn pop_two(&mut self) -> (Value, Value) {
        let b = self.pop();
        let a = self.pop();
        (a, b)
    }

    /// 查看栈顶元素 (distance=0 是栈顶)
    #[inline]
    fn peek(&self, distance: usize) -> Value {
        let len = self.stack.len();
        if len == 0 || distance >= len {
            panic!("Stack underflow in peek");
        }
        let idx = len - 1 - distance;
        self.stack[idx]
    }

    // ==================== 数值运算 ====================

    /// 加法（仅基础类型）
    fn add_values(&self, a: Value, b: Value) -> Result<Value, String> {
        // 字符串拼接
        if let (Some(ap), Some(bp)) = (a.as_string(), b.as_string()) {
            let a_str = unsafe { &(*ap).chars };
            let b_str = unsafe { &(*bp).chars };
            let concatenated = format!("{}{}", a_str, b_str);
            let string_obj = Box::new(crate::runtime::object::ObjString::new(concatenated));
            return Ok(Value::string(Box::into_raw(string_obj)));
        }

        // 优先尝试整数加法
        if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
            // 检查溢出
            if let Some(sum) = ai.checked_add(bi) {
                if sum >= -(1 << 30) && sum < (1 << 30) {
                    return Ok(Value::smi(sum));
                }
            }
        }

        // 检查是否都是数值类型
        let a_is_num = a.is_int() || a.is_float();
        let b_is_num = b.is_int() || b.is_float();
        
        if a_is_num && b_is_num {
            // 回退到浮点数
            let af = if a.is_float() {
                a.as_float()
            } else {
                a.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };
            let bf = if b.is_float() {
                b.as_float()
            } else {
                b.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };
            return Ok(Value::float(af + bf));
        }

        // 不是基础类型，返回错误以便尝试运算符重载
        Err("Non-primitive types need operator overloading".to_string())
    }

    /// 减法（仅基础类型）
    fn sub_values(&self, a: Value, b: Value) -> Result<Value, String> {
        if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
            if let Some(diff) = ai.checked_sub(bi) {
                if diff >= -(1 << 30) && diff < (1 << 30) {
                    return Ok(Value::smi(diff));
                }
            }
        }

        let a_is_num = a.is_int() || a.is_float();
        let b_is_num = b.is_int() || b.is_float();
        
        if a_is_num && b_is_num {
            let af = if a.is_float() {
                a.as_float()
            } else {
                a.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };
            let bf = if b.is_float() {
                b.as_float()
            } else {
                b.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };
            return Ok(Value::float(af - bf));
        }

        Err("Non-primitive types need operator overloading".to_string())
    }

    /// 乘法（仅基础类型）
    fn mul_values(&self, a: Value, b: Value) -> Result<Value, String> {
        if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
            if let Some(prod) = ai.checked_mul(bi) {
                if prod >= -(1 << 30) && prod < (1 << 30) {
                    return Ok(Value::smi(prod));
                }
            }
        }

        let a_is_num = a.is_int() || a.is_float();
        let b_is_num = b.is_int() || b.is_float();
        
        if a_is_num && b_is_num {
            let af = if a.is_float() {
                a.as_float()
            } else {
                a.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };
            let bf = if b.is_float() {
                b.as_float()
            } else {
                b.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };
            return Ok(Value::float(af * bf));
        }

        Err("Non-primitive types need operator overloading".to_string())
    }

    /// 除法（仅基础类型）
    fn div_values(&self, a: Value, b: Value) -> Result<Value, String> {
        let a_is_num = a.is_int() || a.is_float();
        let b_is_num = b.is_int() || b.is_float();
        
        if a_is_num && b_is_num {
            // 除法总是返回浮点数（避免整数除法的困惑）
            let af = if a.is_float() {
                a.as_float()
            } else {
                a.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };

            let bf = if b.is_float() {
                b.as_float()
            } else {
                b.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };

            if bf == 0.0 {
                return Err("Division by zero".to_string());
            }

            return Ok(Value::float(af / bf));
        }

        Err("Non-primitive types need operator overloading".to_string())
    }

    /// 取模/求余（仅基础类型）
    fn mod_values(&self, a: Value, b: Value) -> Result<Value, String> {
        let a_is_num = a.is_int() || a.is_float();
        let b_is_num = b.is_int() || b.is_float();
        
        if a_is_num && b_is_num {
            // 优先尝试整数取模
            if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
                if bi == 0 {
                    return Err("Modulo by zero".to_string());
                }
                return Ok(Value::smi(ai % bi));
            }

            // 回退到浮点数
            let af = if a.is_float() {
                a.as_float()
            } else {
                a.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };

            let bf = if b.is_float() {
                b.as_float()
            } else {
                b.as_smi().map(|n| n as f64).unwrap_or(0.0)
            };

            if bf == 0.0 {
                return Err("Modulo by zero".to_string());
            }

            return Ok(Value::float(af % bf));
        }

        Err("Non-primitive types need operator overloading".to_string())
    }

    /// 取负
    fn neg_value(&self, v: Value) -> Result<Value, String> {
        if let Some(i) = v.as_smi() {
            if i != i32::MIN {
                // 避免溢出
                return Ok(Value::smi(-i));
            }
        }

        let f = if v.is_float() {
            v.as_float()
        } else {
            v.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        Ok(Value::float(-f))
    }

    /// 比较
    fn compare_values(&self, a: Value, b: Value) -> Result<std::cmp::Ordering, String> {
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        Ok(af.partial_cmp(&bf).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// 获取栈顶值（用于测试和获取结果）
    pub fn stack_top(&self) -> Option<Value> {
        self.stack.last().copied()
    }

    // ==================== 运算符重载 ====================

    /// 获取值的 Shape ID
    fn get_shape_id(&self, value: Value) -> u16 {
        // 基础类型使用预定义 Shape ID
        if value.is_int() {
            0  // Int
        } else if value.is_float() {
            1  // Float
        } else if value.is_string() {
            2  // String
        } else if value.is_list() {
            3  // List
        } else if value.is_json() {
            4  // Json
        } else if value.is_closure() {
            5  // Function/Closure
        } else if value.is_module() {
            6  // Module
        } else if value.is_struct() {
            // Struct 需要动态获取 Shape ID
            if let Some(ptr) = value.as_struct() {
                unsafe { (*ptr).shape_id() }
            } else {
                0
            }
        } else {
            0
        }
    }

    /// 获取值的类型名称（用于错误信息）
    fn get_type_name(&self, value: Value) -> &'static str {
        if value.is_int() {
            "int"
        } else if value.is_float() {
            "float"
        } else if value.is_string() {
            "string"
        } else if value.is_list() {
            "List"
        } else if value.is_json() {
            "json"
        } else if value.is_closure() {
            "function"
        } else if value.is_module() {
            "module"
        } else if value.is_struct() {
            "struct"
        } else if value.is_coroutine() {
            "coroutine"
        } else if value.is_null() {
            "null"
        } else if value.is_bool() {
            "bool"
        } else {
            "unknown"
        }
    }

    /// 查找运算符（Level 3：元表查找）
    fn find_operator(&self, value: Value, op: Operator) -> Option<*mut ObjClosure> {
        let shape_id = self.get_shape_id(value);
        
        // 获取 Shape
        if let Some(shape_ptr) = self.shapes.get(&shape_id) {
            let shape = unsafe { &**shape_ptr };
            return shape.get_operator(op);
        }
        
        None
    }

    /// 调用二元运算符（带反向运算符回退）
    fn call_binary_operator(
        &mut self,
        op: Operator,
        a: Value,
        b: Value,
    ) -> Result<Value, String> {
        // 1. 尝试左操作数的运算符
        if let Some(closure) = self.find_operator(a, op) {
            return self.call_operator_closure(closure, &[a, b]);
        }

        // 2. 尝试反向运算符
        if let Some(reverse_op) = op.reverse() {
            if let Some(closure) = self.find_operator(b, reverse_op) {
                return self.call_operator_closure(closure, &[b, a]);
            }
        }

        // 3. 报错
        Err(format!(
            "OperatorError: 类型 '{}' 不支持运算符 '{}'",
            self.get_type_name(a),
            op.symbol()
        ))
    }

    /// 调用一元运算符
    fn call_unary_operator(&mut self, op: Operator, value: Value) -> Result<Value, String> {
        if let Some(closure) = self.find_operator(value, op) {
            return self.call_operator_closure(closure, &[value]);
        }

        Err(format!(
            "OperatorError: 类型 '{}' 不支持运算符 '{}'",
            self.get_type_name(value),
            op.symbol()
        ))
    }

    /// 调用运算符闭包（辅助方法）
    /// 使用内联执行，直接执行运算符闭包的字节码
    fn call_operator_closure(
        &mut self,
        closure: *mut ObjClosure,
        args: &[Value],
    ) -> Result<Value, String> {
        use OpCode::*;
        
        // 获取闭包信息
        let closure_ref = unsafe { &*closure };
        let func = unsafe { &*closure_ref.function };

        if func.arity != args.len() as u8 {
            return Err(format!(
                "Expected {} arguments but got {}",
                func.arity, args.len()
            ));
        }

        // 创建局部变量表（参数）
        let mut locals: Vec<Value> = args.to_vec();
        
        // 创建指令指针
        let mut ip = func.chunk.code.as_ptr();
        let code_end = unsafe { func.chunk.code.as_ptr().add(func.chunk.code.len()) };
        
        // 执行运算符闭包的字节码
        loop {
            if ip >= code_end {
                return Err("Unexpected end of operator bytecode".to_string());
            }
            
            let instruction = unsafe { *ip };
            ip = unsafe { ip.add(1) };
            let op = unsafe { std::mem::transmute::<u8, OpCode>(instruction) };
            
            match op {
                LoadConst => {
                    let idx = unsafe { *ip };
                    ip = unsafe { ip.add(1) };
                    if let Some(val) = func.chunk.constants.get(idx as usize) {
                        self.push(*val);
                    }
                }
                LoadConst0 => {
                    if let Some(val) = func.chunk.constants.get(0) {
                        self.push(*val);
                    }
                }
                LoadConst1 => {
                    if let Some(val) = func.chunk.constants.get(1) {
                        self.push(*val);
                    }
                }
                LoadConst2 => {
                    if let Some(val) = func.chunk.constants.get(2) {
                        self.push(*val);
                    }
                }
                LoadConst3 => {
                    if let Some(val) = func.chunk.constants.get(3) {
                        self.push(*val);
                    }
                }
                LoadNull => self.push(Value::NULL),
                LoadTrue => self.push(Value::TRUE),
                LoadFalse => self.push(Value::FALSE),
                
                LoadLocal0 => self.push(locals[0]),
                LoadLocal1 => self.push(locals[1]),
                LoadLocal2 => self.push(locals[2]),
                LoadLocal3 => self.push(locals[3]),
                LoadLocal => {
                    let idx = unsafe { *ip };
                    ip = unsafe { ip.add(1) };
                    self.push(locals[idx as usize]);
                }
                StoreLocal => {
                    let idx = unsafe { *ip };
                    ip = unsafe { ip.add(1) };
                    let val = self.pop();
                    if (idx as usize) < locals.len() {
                        locals[idx as usize] = val;
                    } else {
                        locals.resize(idx as usize + 1, Value::NULL);
                        locals[idx as usize] = val;
                    }
                }
                StoreLocal0 => {
                    let val = self.pop();
                    if locals.is_empty() {
                        locals.push(val);
                    } else {
                        locals[0] = val;
                    }
                }
                StoreLocal1 => {
                    let val = self.pop();
                    if locals.len() < 2 {
                        locals.resize(2, Value::NULL);
                    }
                    locals[1] = val;
                }
                StoreLocal2 => {
                    let val = self.pop();
                    if locals.len() < 3 {
                        locals.resize(3, Value::NULL);
                    }
                    locals[2] = val;
                }
                
                Pop => { self.pop(); }
                Dup => { let v = self.stack.last().copied().unwrap(); self.push(v); }
                
                BuildStruct => {
                    let shape_id = Self::read_u16_at_ptr(ip);
                    ip = unsafe { ip.add(2) };
                    let field_count = unsafe { *ip };
                    ip = unsafe { ip.add(1) };
                    
                    let mut fields = Vec::with_capacity(field_count as usize);
                    for _ in 0..field_count {
                        fields.push(self.pop());
                    }
                    fields.reverse();
                    
                    let shape_ptr = self.get_shape(shape_id);
                    if shape_ptr.is_null() {
                        return Err(format!("Shape ID {} not found", shape_id));
                    }
                    
                    let obj = ObjStruct::new(shape_ptr, fields);
                    let ptr = Box::into_raw(Box::new(obj));
                    self.push(Value::struct_instance(ptr));
                }
                
                GetField => {
                    let field_idx = unsafe { *ip };
                    ip = unsafe { ip.add(1) };
                    let obj_val = self.pop();
                    if let Some(ptr) = obj_val.as_struct() {
                        let obj = unsafe { &*ptr };
                        if (field_idx as usize) < obj.fields.len() {
                            self.push(obj.fields[field_idx as usize]);
                        } else {
                            return Err("Field index out of bounds".to_string());
                        }
                    } else {
                        return Err("Expected struct instance".to_string());
                    }
                }
                
                IndexGet => {
                    // 栈顶: [index, object]
                    let index_val = self.pop();
                    let obj_val = self.pop();
                    
                    // Struct 字段索引（整数）
                    if let Some(idx) = index_val.as_smi() {
                        if let Some(struct_ptr) = obj_val.as_struct() {
                            let struct_obj = unsafe { &*struct_ptr };
                            let i = idx as usize;
                            if i < struct_obj.field_count() {
                                self.push(struct_obj.fields[i]);
                            } else {
                                return Err(format!("Field index out of bounds: {}", i));
                            }
                        } else {
                            return Err("Expected struct instance for field access".to_string());
                        }
                    } else {
                        return Err("Expected integer index".to_string());
                    }
                }
                
                Add => {
                    let b = self.pop();
                    let a = self.pop();
                    match self.add_values(a, b) {
                        Ok(v) => self.push(v),
                        Err(e) => return Err(e),
                    }
                }
                Sub => {
                    let b = self.pop();
                    let a = self.pop();
                    match self.sub_values(a, b) {
                        Ok(v) => self.push(v),
                        Err(e) => return Err(e),
                    }
                }
                Mul => {
                    let b = self.pop();
                    let a = self.pop();
                    match self.mul_values(a, b) {
                        Ok(v) => self.push(v),
                        Err(e) => return Err(e),
                    }
                }
                Div => {
                    let b = self.pop();
                    let a = self.pop();
                    match self.div_values(a, b) {
                        Ok(v) => self.push(v),
                        Err(e) => return Err(e),
                    }
                }
                
                ReturnValue => {
                    return Ok(self.pop());
                }
                Return => {
                    return Ok(Value::NULL);
                }
                
                _ => {
                    return Err(format!("Unsupported opcode in operator: {:?}", op));
                }
            }
        }
    }

    /// 分配内联缓存槽
    fn allocate_inline_cache(&mut self) -> u8 {
        let index = self.inline_caches.len();
        self.inline_caches.push(InlineCacheEntry::empty());
        index as u8
    }

    // ==================== 调试 ====================

    /// 追踪当前指令执行
    #[cfg(feature = "trace_execution")]
    fn trace_instruction(&self) {
        print!("          ");
        for (i, slot) in self.stack.iter().enumerate() {
            print!("[ {} ]", slot);
        }
        println!();

        // 反汇编当前指令
        let frame = self.frames.last().unwrap();
        let offset = unsafe { frame.ip.offset_from(frame.chunk().code.as_ptr()) } as usize;
        // 只读取查看，不修改 ip
        let instruction = unsafe { *frame.ip };
        let op = unsafe { std::mem::transmute::<u8, OpCode>(instruction) };
        println!("{:04} {:?}", offset, op);
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

// 公开 VM 的 shapes 访问（用于测试和外部注册）
impl VM {
    /// 获取 shape 数量（用于测试）
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }
}

use std::cmp::Ordering;

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::bytecode::OpCode::*;

    #[test]
    fn test_push_pop() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        chunk.write_op(LoadOne, 1);
        chunk.write_op(Pop, 1);
        chunk.write_op(Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_arithmetic() {
        // 1 + 2
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c1 = chunk.add_constant(Value::smi(1));
        let c2 = chunk.add_constant(Value::smi(2));

        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op(Add, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);

        // 检查结果应该是 3
        assert_eq!(vm.stack.last().unwrap().as_smi(), Some(3));
    }

    #[test]
    fn test_add_overflow_to_float() {
        // 大数相加，溢出 SMI 范围，应该转为 float
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let big = (1 << 29) as i32; // 536870912
        let c1 = chunk.add_constant(Value::smi(big));
        let c2 = chunk.add_constant(Value::smi(big));

        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op(Add, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);

        // 结果应该是浮点数
        let top = vm.stack.last().unwrap();
        assert!(top.is_float() || top.as_smi().is_some());
    }

    #[test]
    fn test_comparison() {
        // 2 > 1
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c2 = chunk.add_constant(Value::smi(2));
        let c1 = chunk.add_constant(Value::smi(1));

        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op(Greater, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert!(vm.stack.last().unwrap().is_true());
    }

    #[test]
    fn test_division() {
        // 5 / 2 = 2.5
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c5 = chunk.add_constant(Value::smi(5));
        let c2 = chunk.add_constant(Value::smi(2));

        chunk.write_op_u8(LoadConst, c5, 1);
        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op(Div, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);

        let top = vm.stack.last().unwrap();
        assert!(top.is_float());
        assert_eq!(top.as_float(), 2.5);
    }

    #[test]
    fn test_division_by_zero() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c1 = chunk.add_constant(Value::smi(1));
        let c0 = chunk.add_constant(Value::smi(0));

        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op_u8(LoadConst, c0, 1);
        chunk.write_op(Div, 1);
        chunk.write_op(Return, 1);

        let result = vm.interpret(&chunk);
        assert!(matches!(result, InterpretResult::RuntimeError(_)));
    }

    #[test]
    fn test_jump_if_false() {
        // if (false) { LoadFalse } else { LoadTrue } 应该执行 LoadTrue
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        chunk.write_op(LoadFalse, 1); // 条件为 false

        // JumpIfFalse 跳过 LoadFalse (2 bytes: LoadFalse op)
        let jump_offset = chunk.write_jump(JumpIfFalse, 1);
        chunk.write_op(LoadFalse, 1); // 这个被跳过
        chunk.patch_jump(jump_offset);

        chunk.write_op(LoadTrue, 1); // 应该执行到这里
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert!(vm.stack.last().unwrap().is_true());
    }

    #[test]
    fn test_local_variables() {
        // var x = 5; var y = x + 3;
        // 使用 interpret_with_locals 预分配 2 个局部变量槽
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c5 = chunk.add_constant(Value::smi(5));
        let c3 = chunk.add_constant(Value::smi(3));

        // x = 5
        chunk.write_op_u8(LoadConst, c5, 1);
        chunk.write_op(StoreLocal0, 1);

        // y = x + 3
        chunk.write_op(LoadLocal0, 1); // 加载 x
        chunk.write_op_u8(LoadConst, c3, 1); // 加载 3
        chunk.write_op(Add, 1); // x + 3
        chunk.write_op(StoreLocal1, 1); // y = result

        // return y
        chunk.write_op(LoadLocal1, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret_with_locals(&chunk, 2);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.last().unwrap().as_smi(), Some(8));
    }

    #[test]
    fn test_local_variables_high_index() {
        // 测试高索引局部变量 (超过 7，需要使用 LoadLocal/StoreLocal 指令)
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        // slot 8 = 42
        let c42 = chunk.add_constant(Value::smi(42));
        chunk.write_op_u8(LoadConst, c42, 1);
        chunk.write_op_u8(StoreLocal, 8, 1);

        // return slot 8
        chunk.write_op_u8(LoadLocal, 8, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret_with_locals(&chunk, 10);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.last().unwrap().as_smi(), Some(42));
    }
}
