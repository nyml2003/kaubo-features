pub struct GcHeap {
    slots: Vec<GcSlot>,
    free_list: Vec<usize>,
}

struct GcSlot {
    rc: u32,
    obj: Option<crate::execute::HeapObj>,
}

impl GcHeap {
    pub fn new() -> Self {
        GcHeap {
            slots: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn alloc(&mut self, obj: crate::execute::HeapObj) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.slots[idx] = GcSlot {
                rc: 1,
                obj: Some(obj),
            };
            return idx;
        }
        let idx = self.slots.len();
        self.slots.push(GcSlot {
            rc: 1,
            obj: Some(obj),
        });
        idx
    }

    pub fn retain(&mut self, idx: usize) {
        if idx < self.slots.len() {
            if self.slots[idx].obj.is_some() {
                self.slots[idx].rc += 1;
            }
        }
    }

    pub fn release(&mut self, idx: usize) {
        if idx >= self.slots.len() {
            return;
        }
        if self.slots[idx].obj.is_none() {
            return;
        }
        self.slots[idx].rc -= 1;
        if self.slots[idx].rc == 0 {
            self.slots[idx].obj = None;
            self.free_list.push(idx);
        }
    }

    pub fn get(&self, idx: usize) -> &crate::execute::HeapObj {
        self.slots[idx]
            .obj
            .as_ref()
            .expect("gc_heap: get on empty slot")
    }

    pub fn get_mut(&mut self, idx: usize) -> &mut crate::execute::HeapObj {
        self.slots[idx]
            .obj
            .as_mut()
            .expect("gc_heap: get_mut on empty slot")
    }

    pub fn try_get(&self, idx: usize) -> Option<&crate::execute::HeapObj> {
        self.slots.get(idx).and_then(|s| s.obj.as_ref())
    }

    #[cfg(test)]
    pub fn ref_count(&self, idx: usize) -> u32 {
        self.slots[idx].rc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execute::HeapObj;

    fn int_obj(n: i64) -> HeapObj {
        HeapObj::Struct(0, vec![n])
    }
    fn str_obj(s: &str) -> HeapObj {
        HeapObj::String(s.to_string())
    }

    #[test]
    fn t1_alloc_sets_rc_to_one() {
        let mut heap = GcHeap::new();
        let idx = heap.alloc(str_obj("hello"));
        assert_eq!(heap.ref_count(idx), 1);
        match heap.get(idx) {
            HeapObj::String(s) => assert_eq!(s, "hello"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn t2_retain_increments_rc() {
        let mut heap = GcHeap::new();
        let idx = heap.alloc(int_obj(42));
        heap.retain(idx);
        assert_eq!(heap.ref_count(idx), 2);
    }

    #[test]
    fn t3_release_decrements_rc() {
        let mut heap = GcHeap::new();
        let idx = heap.alloc(int_obj(42));
        heap.retain(idx);
        heap.release(idx);
        assert_eq!(heap.ref_count(idx), 1);
        assert!(matches!(heap.get(idx), HeapObj::Struct(..)));
    }

    #[test]
    #[should_panic(expected = "empty slot")]
    fn t4_release_frees_slot_at_rc_one() {
        let mut heap = GcHeap::new();
        let idx = heap.alloc(int_obj(42));
        heap.release(idx);
        heap.get(idx); // should panic
    }

    #[test]
    fn t5_freed_slot_gets_reused() {
        let mut heap = GcHeap::new();
        let idx1 = heap.alloc(str_obj("first"));
        heap.release(idx1);
        let idx2 = heap.alloc(str_obj("second"));
        assert_eq!(idx1, idx2, "freed slot should be reused");
        match heap.get(idx2) {
            HeapObj::String(s) => assert_eq!(s, "second"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn t6_multiple_refs_keeps_alive() {
        let mut heap = GcHeap::new();
        let idx = heap.alloc(int_obj(42));
        heap.retain(idx);
        heap.retain(idx);
        heap.retain(idx);
        assert_eq!(heap.ref_count(idx), 4);
        heap.release(idx); // rc=3
        heap.release(idx); // rc=2
        heap.release(idx); // rc=1
        assert_eq!(heap.ref_count(idx), 1);
        assert!(matches!(heap.get(idx), HeapObj::Struct(..)));
        heap.release(idx); // rc=0 -> frees
        assert_eq!(heap.ref_count(idx), 0);
    }

    #[test]
    fn t7_release_out_of_bounds_safe() {
        let mut heap = GcHeap::new();
        heap.release(99999);
        // 不 panic 即通过
    }

    #[test]
    fn t8_release_already_freed_idempotent() {
        let mut heap = GcHeap::new();
        let idx = heap.alloc(int_obj(42));
        heap.release(idx);
        heap.release(idx); // 第二次 release 同一空槽，不应 panic
    }
}
