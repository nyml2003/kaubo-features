use crate::runtime::{
    type_info::{TypeInfo, ValueTypeInfo, ValueTypeKind},
    value::{Value, ValueUnion},
};

/// 堆上存储的引用类型对象
#[derive(Debug, Clone, PartialEq)]
pub enum HeapObject {
    Array(Vec<Value>),
    Dictionary(Vec<(Value, Value)>),
    String(String),
}

/// 堆相关错误（扩展自ValueError）
#[derive(Debug, PartialEq, Clone)]
pub enum HeapError {
    ObjectLimitExceeded(usize, usize), // 最大对象数 + 尝试分配数
    MemoryLimitExceeded(usize, usize), // 最大内存(字节) + 尝试分配内存
    InvalidPointer(usize),             // 无效指针（已回收或越界）
    WrongObjectType,                   // 对象类型不匹配（如数组预期却拿到字符串）
}

/// 带垃圾回收的堆结构
#[derive(Debug, Default)]
pub struct GcHeap {
    // 核心存储：(对象数据, 标记状态)
    objects: Vec<(HeapObject, bool)>,
    // 空闲索引管理
    free_indices: Vec<usize>,
    is_free: Vec<bool>, // 快速判断索引是否空闲
    // 堆上限配置
    max_object_count: usize,
    max_memory_size: usize,
}

impl GcHeap {
    /// 自定义堆上限的构造器
    pub fn with_limits(max_object_count: usize, max_memory_size: usize) -> Self {
        Self {
            max_object_count,
            max_memory_size,
            ..Default::default()
        }
    }

    /// 默认堆上限（10万对象，1GB内存）
    pub fn default_limits() -> Self {
        Self::with_limits(100_000, 1024 * 1024 * 1024) // 1GB
    }

    /// 计算单个对象的内存占用（简化版，实际可更精确）
    fn object_memory_size(obj: &HeapObject) -> usize {
        match obj {
            HeapObject::Array(elems) => {
                std::mem::size_of::<Vec<Value>>()
                    + elems.iter().map(|v| v.type_info.size()).sum::<usize>()
            }
            HeapObject::Dictionary(items) => {
                std::mem::size_of::<Vec<(Value, Value)>>()
                    + items
                        .iter()
                        .map(|(k, v)| k.type_info.size() + v.type_info.size())
                        .sum::<usize>()
            }
            HeapObject::String(s) => std::mem::size_of::<String>() + s.as_bytes().len(),
        }
    }

    /// 计算当前堆中活跃对象的总内存
    fn total_memory_used(&self) -> usize {
        self.objects
            .iter()
            .enumerate()
            .filter(|(idx, _)| !self.is_free[*idx])
            .map(|(_, (obj, _))| Self::object_memory_size(obj))
            .sum()
    }

    /// 计算当前活跃对象数量
    fn active_object_count(&self) -> usize {
        self.objects.len() - self.free_indices.len()
    }

    /// 分配对象到堆上（返回指针，失败返回错误）
    pub fn alloc(&mut self, obj: HeapObject, roots: &[Value]) -> Result<usize, HeapError> {
        let new_obj_size = Self::object_memory_size(&obj);
        let current_count = self.active_object_count();
        let current_memory = self.total_memory_used();

        // 检查是否超上限，超则触发GC
        let would_exceed_count = current_count + 1 > self.max_object_count;
        let would_exceed_memory = current_memory + new_obj_size > self.max_memory_size;

        if would_exceed_count || would_exceed_memory {
            self.collect(roots); // 触发GC回收

            // GC后再次检查
            let after_count = self.active_object_count();
            let after_memory = self.total_memory_used();
            if after_count + 1 > self.max_object_count {
                return Err(HeapError::ObjectLimitExceeded(
                    self.max_object_count,
                    after_count + 1,
                ));
            }
            if after_memory + new_obj_size > self.max_memory_size {
                return Err(HeapError::MemoryLimitExceeded(
                    self.max_memory_size,
                    after_memory + new_obj_size,
                ));
            }
        }

        // 分配对象（复用空闲索引或新增）
        let ptr = if let Some(idx) = self.free_indices.pop() {
            self.objects[idx] = (obj, false);
            self.is_free[idx] = false;
            idx
        } else {
            self.objects.push((obj, false));
            self.is_free.push(false);
            self.objects.len() - 1
        };

        Ok(ptr)
    }

    /// 通过指针获取不可变对象引用
    pub fn get(&self, ptr: usize) -> Result<&HeapObject, HeapError> {
        if ptr >= self.objects.len() || self.is_free[ptr] {
            return Err(HeapError::InvalidPointer(ptr));
        }
        Ok(&self.objects[ptr].0)
    }

    /// 通过指针获取可变对象引用
    pub fn get_mut(&mut self, ptr: usize) -> Result<&mut HeapObject, HeapError> {
        if ptr >= self.objects.len() || self.is_free[ptr] {
            return Err(HeapError::InvalidPointer(ptr));
        }
        Ok(&mut self.objects[ptr].0)
    }

    /// 标记阶段：标记所有可达对象
    fn mark(&mut self, roots: &[Value]) {
        // 重置所有标记
        for (_, marked) in &mut self.objects {
            *marked = false;
        }

        let mut stack = Vec::new();
        // 收集根对象中的引用
        for root in roots {
            if let ValueUnion::Reference(ptr) = &root.value {
                let ptr = *ptr;
                if ptr < self.objects.len() && !self.is_free[ptr] {
                    stack.push(ptr);
                }
            }
        }

        // 迭代标记所有可达对象
        while let Some(ptr) = stack.pop() {
            // 检查是否已经标记过
            if self.objects[ptr].1 {
                continue;
            }

            // 标记当前对象
            self.objects[ptr].1 = true;

            // 递归标记对象内部的引用
            match &self.objects[ptr].0 {
                HeapObject::Array(elems) => {
                    for elem in elems {
                        if let ValueUnion::Reference(elem_ptr) = &elem.value {
                            let elem_ptr = *elem_ptr;
                            if elem_ptr < self.objects.len()
                                && !self.is_free[elem_ptr]
                                && !self.objects[elem_ptr].1
                            {
                                stack.push(elem_ptr);
                            }
                        }
                    }
                }
                HeapObject::Dictionary(items) => {
                    for (k, v) in items {
                        for val in [k, v].iter() {
                            if let ValueUnion::Reference(ptr) = &val.value {
                                let ptr = *ptr;
                                if ptr < self.objects.len()
                                    && !self.is_free[ptr]
                                    && !self.objects[ptr].1
                                {
                                    stack.push(ptr);
                                }
                            }
                        }
                    }
                }
                HeapObject::String(_) => {} // 字符串无内部引用
            }
        }
    }

    /// 清除阶段：回收未标记对象
    fn sweep(&mut self) {
        let mut new_free = Vec::new();
        for (idx, (_, marked)) in self.objects.iter().enumerate() {
            if !*marked && !self.is_free[idx] {
                new_free.push(idx); // 未标记且非空闲的对象加入空闲池
            }
        }
        // 更新空闲索引（保留原有未被回收的空闲索引）
        self.free_indices.extend(new_free);
        // 更新is_free标记
        for &idx in &self.free_indices {
            self.is_free[idx] = true;
        }
    }

    /// 执行垃圾回收（标记+清除）
    pub fn collect(&mut self, roots: &[Value]) {
        self.mark(roots);
        self.sweep();
    }
}

// 适配之前的Heap trait（用于索引访问）
pub trait Heap {
    fn get_array_elements(&self, ptr: usize) -> Result<&[Value], HeapError>;
    fn get_dictionary_items(&self, ptr: usize) -> Result<&[(Value, Value)], HeapError>;
}

impl Heap for GcHeap {
    fn get_array_elements(&self, ptr: usize) -> Result<&[Value], HeapError> {
        match self.get(ptr)? {
            HeapObject::Array(elems) => Ok(elems),
            _ => Err(HeapError::WrongObjectType),
        }
    }

    fn get_dictionary_items(&self, ptr: usize) -> Result<&[(Value, Value)], HeapError> {
        match self.get(ptr)? {
            HeapObject::Dictionary(items) => Ok(items),
            _ => Err(HeapError::WrongObjectType),
        }
    }
}

fn build_int32(value: i32) -> Value {
    let int32_type = TypeInfo::Value(ValueTypeInfo {
        kind: ValueTypeKind::Int32,
    });
    let int32_value = Value {
        type_info: int32_type,
        value: ValueUnion::Int32(value),
    };
    int32_value
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_basic_allocation_and_access() {
        let mut heap = GcHeap::default_limits();
        let roots = Vec::new();

        // 分配一个数组对象
        let array = HeapObject::Array(vec![build_int32(10), build_int32(20)]);
        let ptr = heap.alloc(array, &roots).unwrap();

        // 访问数组
        let obj = heap.get(ptr).unwrap();
        assert!(matches!(obj, HeapObject::Array(elems) if elems.len() == 2));

        // 验证数组元素
        if let HeapObject::Array(elems) = obj {
            assert_eq!(elems[0].value, ValueUnion::Int32(10));
            assert_eq!(elems[1].value, ValueUnion::Int32(20));
        }
    }

    #[test]
    fn test_memory_limit() {
        // 设置小内存限制（100字节）
        let mut heap = GcHeap::with_limits(100, 100);
        let roots = Vec::new();

        // 分配一个大字符串（超过100字节）
        let big_str = "a".repeat(200); // 200字节
        let obj = HeapObject::String(big_str);

        // 首次分配会触发GC（但无效果），然后返回内存超限错误
        let result = heap.alloc(obj, &roots);
        assert!(matches!(
            result,
            Err(HeapError::MemoryLimitExceeded(100, _))
        ));
    }

    #[test]
    fn test_invalid_pointer() {
        let mut heap = GcHeap::default_limits();
        let roots = Vec::new();

        // 分配一个对象后回收
        let ptr = heap
            .alloc(HeapObject::String("test".into()), &roots)
            .unwrap();
        heap.collect(&[]); // 根集为空，对象被回收

        // 访问已回收的指针
        assert!(matches!(heap.get(ptr), Err(HeapError::InvalidPointer(_))));
        // 访问越界指针
        assert!(matches!(heap.get(9999), Err(HeapError::InvalidPointer(_))));
    }

    #[test]
    fn test_wrong_object_type() {
        let mut heap = GcHeap::default_limits();
        let roots = Vec::new();

        // 分配字符串对象
        let str_ptr = heap
            .alloc(HeapObject::String("hello".into()), &roots)
            .unwrap();
        // 尝试按数组访问（类型不匹配）
        assert!(matches!(
            heap.get_array_elements(str_ptr),
            Err(HeapError::WrongObjectType)
        ));

        // 分配数组对象
        let arr_ptr = heap.alloc(HeapObject::Array(vec![]), &roots).unwrap();
        // 尝试按字典访问（类型不匹配）
        assert!(matches!(
            heap.get_dictionary_items(arr_ptr),
            Err(HeapError::WrongObjectType)
        ));
    }

    #[test]
    fn test_dictionary_and_string() {
        let mut heap = GcHeap::default_limits();
        let roots = Vec::new();

        // 测试字典
        let dict = HeapObject::Dictionary(vec![
            (build_int32(1), build_int32(100)),
            (build_int32(2), build_int32(200)),
        ]);
        let dict_ptr = heap.alloc(dict, &roots).unwrap();
        let items = heap.get_dictionary_items(dict_ptr).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0.value, ValueUnion::Int32(1));
        assert_eq!(items[0].1.value, ValueUnion::Int32(100));

        // 测试字符串
        let s = HeapObject::String("test string".to_string());
        let str_ptr = heap.alloc(s, &roots).unwrap();
        let str_obj = heap.get(str_ptr).unwrap();
        assert!(matches!(str_obj, HeapObject::String(s) if s == "test string"));
    }

    #[test]
    fn test_free_index_reuse() {
        let mut heap = GcHeap::default_limits();
        let roots = Vec::new();

        // 分配3个对象
        let ptr1 = heap.alloc(HeapObject::String("1".into()), &roots).unwrap();
        let ptr2 = heap.alloc(HeapObject::String("2".into()), &roots).unwrap();
        let ptr3 = heap.alloc(HeapObject::String("3".into()), &roots).unwrap();
        assert_eq!(ptr1, 0);
        assert_eq!(ptr2, 1);
        assert_eq!(ptr3, 2);

        // 回收ptr2
        heap.collect(&[]); // 根集为空，所有对象被回收
        assert_eq!(heap.free_indices.len(), 3); // 0,1,2都空闲

        // 新分配应该复用最小的空闲索引（0）
        let new_ptr = heap
            .alloc(HeapObject::String("new".into()), &roots)
            .unwrap();
        assert_eq!(new_ptr, 2);
        let obj = heap.get(new_ptr).unwrap();
        assert!(matches!(obj, HeapObject::String(s) if s == "new"));
        let new_ptr2 = heap
            .alloc(
                HeapObject::Array(vec![build_int32(1), build_int32(2)]),
                &roots,
            )
            .unwrap();

        assert_eq!(new_ptr2, 1);
        let obj = heap.get(new_ptr2).unwrap();
        assert!(matches!(obj, HeapObject::Array(elems) if elems.len() == 2));
    }
}
