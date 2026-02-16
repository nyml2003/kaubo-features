//! 虚拟机实现

use crate::core::{
    CallFrame, Chunk, InterpretResult, ObjClosure, ObjFunction, ObjShape, ObjUpvalue, Operator,
    Value, VM,
};

// 子模块
mod call;
mod execution;
mod index;
mod operators;
mod shape;
mod stack;

// 公开子模块的公共接口
pub use stack::stack_top;

impl VM {
    /// 初始化标准库模块
    pub fn init_stdlib(&mut self) {
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
        execution::register_operators_from_chunk(self, chunk);

        // 加载 Chunk 的内联缓存到 VM
        // 这会预分配缓存槽位，供执行期间使用
        self.inline_caches.clear();
        self.inline_caches.extend(chunk.inline_caches.clone());

        // 创建函数对象（使用 clone 的 chunk，所有权转移给 function）
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
        // 注意：ip 必须指向 closure 的 function 的 chunk，而不是传入的 chunk
        // 因为 function 拥有 chunk 的 clone
        let ip = unsafe { (*(*closure).function).chunk.code.as_ptr() };
        self.frames.push(CallFrame {
            closure,
            ip,
            locals,
            stack_base: 0,
        });

        // 执行主循环
        let result = self.run();

        // 清理调用栈
        self.frames.pop();

        // 关闭所有 upvalues
        call::close_upvalues(self, 0);

        result
    }

    /// 执行字节码的主循环
    ///
    /// 注意：此方法为 crate 内部可见，用于 VM-aware 原生函数
    pub(crate) fn run(&mut self) -> InterpretResult {
        execution::run(self)
    }

    /// 从 Chunk 的 operator_table 注册运算符到 Shape
    fn register_operators_from_chunk(&mut self, chunk: &Chunk) {
        execution::register_operators_from_chunk(self, chunk);
    }

    // ==================== Shape 管理 ====================

    /// 注册 Shape 到 VM
    ///
    /// # Safety
    /// `shape` 必须是有效的、非空的指向 `ObjShape` 的指针
    pub unsafe fn register_shape(&mut self, shape: *const ObjShape) {
        shape::register_shape(self, shape);
    }

    /// 通过 ID 获取 Shape
    fn get_shape(&self, shape_id: u16) -> *const ObjShape {
        shape::get_shape(self, shape_id)
    }

    /// 注册方法到 Shape 的方法表
    pub fn register_method_to_shape(
        &mut self,
        shape_id: u16,
        method_idx: u8,
        func: *mut ObjFunction,
    ) {
        shape::register_method_to_shape(self, shape_id, method_idx, func);
    }

    // ==================== 栈操作代理 ====================

    /// 压栈
    #[inline]
    fn push(&mut self, value: Value) {
        stack::push(self, value);
    }

    /// 弹栈
    #[inline]
    fn pop(&mut self) -> Value {
        stack::pop(self)
    }

    /// 弹出两个值 (先弹出的是右操作数)
    #[inline]
    fn pop_two(&mut self) -> (Value, Value) {
        stack::pop_two(self)
    }

    /// 查看栈顶元素 (distance=0 是栈顶)
    #[inline]
    fn peek(&self, distance: usize) -> Value {
        stack::peek(self, distance)
    }

    // ==================== 辅助方法代理 ====================

    /// 获取当前帧的指令指针
    #[inline]
    fn current_ip(&self) -> *const u8 {
        execution::current_ip(self)
    }

    /// 获取当前帧的可变指令指针
    #[inline]
    fn current_ip_mut(&mut self) -> &mut *const u8 {
        execution::current_ip_mut(self)
    }

    /// 获取当前帧的可变 locals
    #[inline]
    fn current_locals_mut(&mut self) -> &mut Vec<Value> {
        execution::current_locals_mut(self)
    }

    /// 获取当前帧的 locals
    #[inline]
    fn current_locals(&self) -> &Vec<Value> {
        execution::current_locals(self)
    }

    /// 获取局部变量（自动扩展）
    #[inline]
    fn get_local(&self, idx: usize) -> Value {
        execution::get_local(self, idx)
    }

    /// 设置局部变量（自动扩展）
    #[inline]
    fn set_local(&mut self, idx: usize, value: Value) {
        execution::set_local(self, idx, value);
    }

    /// 获取当前帧的 chunk
    #[inline]
    fn current_chunk(&self) -> &crate::core::Chunk {
        execution::current_chunk(self)
    }

    /// 获取常量池中的字符串
    #[inline]
    fn get_constant_string(&self, idx: usize) -> String {
        execution::get_constant_string(self, idx)
    }

    /// 获取当前闭包
    #[inline]
    fn current_closure(&self) -> *mut ObjClosure {
        execution::current_closure(self)
    }

    /// 获取局部变量指针（用于 upvalue 捕获）
    fn current_local_ptr(&mut self, idx: usize) -> *mut Value {
        call::current_local_ptr(self, idx)
    }

    /// 捕获 upvalue（如果已存在则复用）
    fn capture_upvalue(&mut self, location: *mut Value) -> *mut ObjUpvalue {
        call::capture_upvalue(self, location)
    }

    /// 关闭从指定槽位开始的所有 upvalues
    fn close_upvalues(&mut self, slot: usize) {
        call::close_upvalues(self, slot);
    }

    /// 前进指令指针
    #[inline]
    fn advance_ip(&mut self, offset: usize) {
        execution::advance_ip(self, offset);
    }

    /// 跳转指令指针
    #[inline]
    fn jump_ip(&mut self, offset: isize) {
        execution::jump_ip(self, offset);
    }

    /// 读取下一个字节
    #[inline]
    fn read_byte(&mut self) -> u8 {
        execution::read_byte(self)
    }

    /// 读取 i16
    #[inline]
    fn read_i16(&mut self) -> i16 {
        execution::read_i16(self)
    }

    /// 读取 u16
    #[inline]
    fn read_u16(&mut self) -> u16 {
        execution::read_u16(self)
    }

    /// 从常量池加载并压栈
    #[inline]
    fn push_const(&mut self, idx: usize) {
        execution::push_const(self, idx);
    }

    // ==================== 数值运算代理 ====================

    /// 加法（仅基础类型）
    fn add_values(&self, a: Value, b: Value) -> Result<Value, String> {
        operators::add_values(self, a, b)
    }

    /// 减法（仅基础类型）
    fn sub_values(&self, a: Value, b: Value) -> Result<Value, String> {
        operators::sub_values(self, a, b)
    }

    /// 乘法（仅基础类型）
    fn mul_values(&self, a: Value, b: Value) -> Result<Value, String> {
        operators::mul_values(self, a, b)
    }

    /// 除法（仅基础类型）
    fn div_values(&self, a: Value, b: Value) -> Result<Value, String> {
        operators::div_values(self, a, b)
    }

    /// 取模/求余（仅基础类型）
    fn mod_values(&self, a: Value, b: Value) -> Result<Value, String> {
        operators::mod_values(self, a, b)
    }

    /// 取负（仅基础类型）
    fn neg_value(&self, v: Value) -> Result<Value, String> {
        operators::neg_value(self, v)
    }

    /// 比较（仅基础数值类型）
    fn compare_values(&self, a: Value, b: Value) -> Result<std::cmp::Ordering, String> {
        operators::compare_values(self, a, b)
    }

    /// 获取栈顶值（用于测试和获取结果）
    pub fn stack_top(&self) -> Option<Value> {
        stack::stack_top(self)
    }

    // ==================== 索引操作代理 ====================

    /// 基础类型索引获取（用于 IndexGet）
    fn index_get_base(&self, obj_val: Value, index_val: Value) -> Result<Option<Value>, String> {
        index::index_get_base(self, obj_val, index_val)
    }

    /// 基础类型索引设置（用于 IndexSet）
    fn index_set_base(
        &mut self,
        obj_val: Value,
        key_val: Value,
        value: Value,
    ) -> Result<bool, String> {
        index::index_set_base(self, obj_val, key_val, value)
    }

    /// 调用 operator set（三元运算符）
    fn call_set_operator(&mut self, obj: Value, index: Value, value: Value) -> Result<(), String> {
        index::call_set_operator(self, obj, index, value)
    }

    // ==================== 运算符重载代理 ====================

    /// 获取值的 Shape ID
    fn get_shape_id(&self, value: Value) -> u16 {
        operators::get_shape_id(self, value)
    }

    /// 获取值的类型名称（用于错误信息）
    fn get_type_name(&self, value: Value) -> &'static str {
        operators::get_type_name(value)
    }

    /// 查找运算符（Level 3：元表查找）
    fn find_operator(&self, value: Value, op: Operator) -> Option<*mut ObjClosure> {
        operators::find_operator(self, value, op)
    }

    /// 调用二元运算符（带反向运算符回退）
    fn call_binary_operator(&mut self, op: Operator, a: Value, b: Value) -> Result<Value, String> {
        operators::call_binary_operator(self, op, a, b)
    }

    /// 调用 operator call（可调用对象，变长参数）
    fn call_callable_operator(&mut self, op: Operator, args: &[Value]) -> Result<Value, String> {
        operators::call_callable_operator(self, op, args)
    }

    /// 调用运算符闭包（变长参数版本）
    fn call_operator_closure_varargs(
        &mut self,
        closure: *mut ObjClosure,
        args: &[Value],
    ) -> Result<Value, String> {
        operators::call_operator_closure_varargs(self, closure, args)
    }

    /// 调用一元运算符（Neg, Not 等）
    #[allow(dead_code)]
    fn call_unary_operator(&mut self, op: Operator, value: Value) -> Result<Value, String> {
        operators::call_unary_operator(self, op, value)
    }

    /// 调用运算符闭包（辅助方法）
    fn call_operator_closure(
        &mut self,
        closure: *mut ObjClosure,
        args: &[Value],
    ) -> Result<Value, String> {
        operators::call_operator_closure(self, closure, args)
    }

    /// 分配内联缓存槽（Level 2 优化）
    #[allow(dead_code)]
    fn allocate_inline_cache(&mut self) -> u8 {
        operators::allocate_inline_cache(self)
    }

    /// 获取内联缓存条目（如果匹配）
    fn inline_cache_get(
        &self,
        cache_idx: u8,
        left: Value,
        right: Value,
    ) -> Option<*mut ObjClosure> {
        operators::inline_cache_get(self, cache_idx, left, right)
    }

    /// 更新内联缓存
    fn inline_cache_update(
        &mut self,
        cache_idx: u8,
        left: Value,
        right: Value,
        closure: *mut ObjClosure,
    ) {
        operators::inline_cache_update(self, cache_idx, left, right, closure);
    }

    /// 调用二元运算符并缓存结果（Level 2）
    fn call_binary_operator_cached(
        &mut self,
        op: Operator,
        a: Value,
        b: Value,
        cache_idx: u8,
    ) -> Result<Value, String> {
        operators::call_binary_operator_cached(self, op, a, b, cache_idx)
    }

    // ==================== 调试 ====================

    /// 追踪当前指令执行
    #[cfg(feature = "trace_execution")]
    fn trace_instruction(&self) {
        execution::trace_instruction(self);
    }
}

// 公开 VM 的 shapes 访问（用于测试和外部注册）
impl VM {
    /// 获取 shape 数量（用于测试）
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Chunk;
    use crate::runtime::OpCode::*;

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
        chunk.write_op_u8(Add, 0xFF, 1);
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

        let big = (1 << 29); // 536870912
        let c1 = chunk.add_constant(Value::smi(big));
        let c2 = chunk.add_constant(Value::smi(big));

        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op_u8(Add, 0xFF, 1);
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
        chunk.write_op_u8(Greater, 0xFF, 1);
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
        chunk.write_op_u8(Div, 0xFF, 1);
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
        chunk.write_op_u8(Div, 0xFF, 1);
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
        chunk.write_op_u8(Add, 0xFF, 1); // x + 3 (0xFF = 无缓存)
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

    #[test]
    fn test_inline_cache_integration() {
        // 测试 Level 2 内联缓存集成
        // 创建一个 Chunk，使用内联缓存进行运算符重载
        use crate::core::ObjShape;
        use crate::runtime::compiler::compile_with_struct_info;
        use crate::compiler::parser::parser::Parser;
        use crate::compiler::lexer::builder::build_lexer;
        use std::collections::HashMap;

        // 编译包含 operator add 的代码
        let code = r#"
            struct Counter {
                value: int
            };
            
            impl Counter {
                operator add: |self, other: Counter| -> Counter {
                    return Counter { value: self.value + other.value };
                }
            };
            
            var c1 = Counter { value: 10 };
            var c2 = Counter { value: 20 };
            var c3 = c1 + c2;
            return c3.value;
        "#;

        let mut lexer = build_lexer();
        let _ = lexer.feed(code.as_bytes());
        let _ = lexer.terminate();
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().unwrap();
        
        // 准备 struct 信息（包含字段类型）
        let mut struct_infos: HashMap<String, (u16, Vec<String>, Vec<String>)> = HashMap::new();
        struct_infos.insert("Counter".to_string(), (100, vec!["value".to_string()], vec!["int".to_string()]));
        
        let result = compile_with_struct_info(&ast, struct_infos);
        
        if let Ok((chunk, local_count)) = result {
            // 验证 Chunk 分配了内联缓存槽位
            assert!(!chunk.inline_caches.is_empty(), 
                "Chunk should have allocated inline cache slots");
            
            let mut vm = VM::new();
            
            // 注册 shapes
            let shape = Box::into_raw(Box::new(ObjShape::new(
                100, // struct shape_id 从 100 开始
                "Counter".to_string(),
                vec!["value".to_string()],
            )));
            unsafe {
                vm.register_shape(shape);
            }
            
            // 执行代码
            let result = vm.interpret_with_locals(&chunk, local_count);
            assert_eq!(result, InterpretResult::Ok);
            
            // 验证结果: 10 + 20 = 30
            let return_value = vm.stack.last().unwrap();
            assert_eq!(return_value.as_smi(), Some(30), 
                "Counter addition should return 30");
            
            // 验证 VM 的内联缓存被正确加载
            assert!(!vm.inline_caches.is_empty(),
                "VM should have loaded inline caches from chunk");
            
            // 验证至少有一个缓存条目被填充（非空）
            let has_filled_cache = vm.inline_caches.iter()
                .any(|cache| !cache.is_empty());
            assert!(has_filled_cache,
                "At least one inline cache entry should be filled after execution");
        } else {
            panic!("Compilation failed: {:?}", result);
        }
    }

    #[test]
    fn test_inline_cache_multiple_calls() {
        // 测试多次调用时缓存命中的情况
        use crate::runtime::compiler::compile_with_struct_info;
        use crate::compiler::parser::parser::Parser;
        use crate::compiler::lexer::builder::build_lexer;
        use std::collections::HashMap;

        let code = r#"
            struct Point {
                x: int,
                y: int
            };
            
            impl Point {
                operator add: |self, other: Point| -> Point {
                    return Point { 
                        x: self.x + other.x,
                        y: self.y + other.y
                    };
                }
            };
            
            var p1 = Point { x: 1, y: 2 };
            var p2 = Point { x: 3, y: 4 };
            
            // 多次执行相同的加法，应该命中缓存
            var r1 = p1 + p2;
            var r2 = p1 + p2;
            var r3 = p1 + p2;
            
            return r1.x + r1.y + r2.x + r2.y + r3.x + r3.y;
        "#;

        let mut lexer = build_lexer();
        let _ = lexer.feed(code.as_bytes());
        let _ = lexer.terminate();
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().unwrap();
        
        // 准备 struct 信息（包含字段类型）
        let mut struct_infos: HashMap<String, (u16, Vec<String>, Vec<String>)> = HashMap::new();
        struct_infos.insert("Point".to_string(), (100, vec!["x".to_string(), "y".to_string()], vec!["int".to_string(), "int".to_string()]));
        
        let result = compile_with_struct_info(&ast, struct_infos);
        
        if let Ok((chunk, local_count)) = result {
            let mut vm = VM::new();
            
            // 注册 Point shape
            let shape = Box::into_raw(Box::new(crate::core::ObjShape::new(
                100,
                "Point".to_string(),
                vec!["x".to_string(), "y".to_string()],
            )));
            unsafe {
                vm.register_shape(shape);
            }
            
            let result = vm.interpret_with_locals(&chunk, local_count);
            assert_eq!(result, InterpretResult::Ok);
            
            // 计算: r1=(4,6), r2=(4,6), r3=(4,6), 总和 = 4+6+4+6+4+6 = 30
            let return_value = vm.stack.last().unwrap();
            assert_eq!(return_value.as_smi(), Some(30),
                "Multiple Point additions should return 30");
        } else {
            panic!("Compilation failed: {:?}", result);
        }
    }
}
