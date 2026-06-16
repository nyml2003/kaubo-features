//! async/await 运行时 — 单线程协作式调度
//!
//! v2.0: 事件循环 + 帧挂起/恢复
//! CPS Suspend terminator → 保存帧 → 调度器接管
//! 预留: IoPoller trait 抽象 I/O (epoll/kqueue/WASM)

use crate::execute::CallFrame;

/// 异步任务 ID
pub type TaskId = usize;

/// 挂起的执行帧
#[derive(Debug, Clone)]
pub struct SuspendedFrame {
    pub frame: CallFrame,
    pub ip: usize,             // 挂起位置的 IP (resume 后从此继续)
}

/// 异步调度器 (v2.0: 简单队列模型)
pub struct AsyncScheduler {
    tasks: Vec<(TaskId, SuspendedFrame)>,
    next_id: TaskId,
    completed: Vec<(TaskId, i64)>,  // 已完成任务的结果
}

impl AsyncScheduler {
    pub fn new() -> Self {
        AsyncScheduler { tasks: vec![], next_id: 0, completed: vec![] }
    }

    /// 注册挂起帧，返回任务 ID
    pub fn suspend(&mut self, frame: CallFrame, ip: usize) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push((id, SuspendedFrame { frame, ip }));
        id
    }

    /// 检查任务是否已完成
    pub fn poll(&mut self) -> Option<(TaskId, i64)> {
        self.completed.pop()
    }

    /// 恢复一个挂起的帧
    pub fn resume_frame(&mut self, task_id: TaskId) -> Option<SuspendedFrame> {
        if let Some(pos) = self.tasks.iter().position(|(id, _)| *id == task_id) {
            Some(self.tasks.swap_remove(pos).1)
        } else {
            None
        }
    }

    /// 将挂起但已完成的帧标记为就绪 (模拟 I/O 完成)
    pub fn complete(&mut self, task_id: TaskId, result: i64) {
        // 从挂起列表移除
        if let Some(pos) = self.tasks.iter().position(|(id, _)| *id == task_id) {
            self.tasks.remove(pos);
        }
        self.completed.push((task_id, result));
    }

    /// 是否有待处理的任务
    pub fn has_pending(&self) -> bool {
        !self.tasks.is_empty()
    }

    /// 获取所有待处理的任务 ID
    pub fn pending_ids(&self) -> Vec<TaskId> {
        self.tasks.iter().map(|(id, _)| *id).collect()
    }

    /// 一次性完成所有待处理任务 (模拟同步 IO)
    pub fn flush_all(&mut self, result: i64) -> Vec<(TaskId, i64)> {
        let mut completed = vec![];
        for (id, _) in std::mem::take(&mut self.tasks) {
            completed.push((id, result));
        }
        self.completed.extend(completed.clone());
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regfile::RegFile;
    use crate::execute::CallFrame;

    #[test]
    fn test_suspend_and_resume() {
        let mut sched = AsyncScheduler::new();
        let cf = CallFrame { int_base: 0, float_base: 0, ptr_base: 0, ptr_count: 0, ret_block: 0, func_entry: 0 };
        let id = sched.suspend(cf, 42);
        assert_eq!(sched.tasks.len(), 1);
        let sf = sched.resume_frame(id).unwrap();
        assert_eq!(sf.ip, 42);
    }

    #[test]
    fn test_complete() {
        let mut sched = AsyncScheduler::new();
        let cf = CallFrame { int_base: 0, float_base: 0, ptr_base: 0, ptr_count: 0, ret_block: 0, func_entry: 0 };
        let id = sched.suspend(cf, 0);
        sched.complete(id, 100);
        assert!(sched.tasks.is_empty());
        assert_eq!(sched.poll(), Some((id, 100)));
    }
}
