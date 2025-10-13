pub type StateId = usize;
/// 事件类型（输入字符）
pub type Event = char;
/// 转移条件：接收事件并返回是否满足条件
pub type TransitionCondition = Box<dyn Fn(Event) -> bool + 'static>;

pub struct State {
    pub is_accepting: bool,
}

/// 转移规则
pub struct Transition {
    pub to: StateId,
    pub condition: TransitionCondition,
}

/// 状态机
pub struct Machine<T> {
    pub states: Vec<State>,
    pub transitions: Vec<Vec<Transition>>,
    pub current_state: StateId,
    pub token: T,
}
