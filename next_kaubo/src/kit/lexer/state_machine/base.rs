use super::common::{Event, Machine, State, StateId, Transition, TransitionCondition};
use super::error::{AddTransitionError, InvalidStateIdError, ProcessEventError};
use std::fmt;
impl<T> Machine<T>
where
    T: fmt::Debug + Clone, // 约束：令牌类型需要实现Debug和Clone
{
    /// 创建新的状态机
    pub fn new(token: T) -> Self {
        let mut states = Vec::new();
        let mut transitions = Vec::new();

        // 添加初始状态
        states.push(State {
            is_accepting: false,
        });
        transitions.push(Vec::new());

        Self {
            states,
            transitions,
            current_state: 0,
            token,
        }
    }

    /// 添加新状态
    /// 返回新状态ID
    pub fn add_state(&mut self, is_accepting: bool) -> StateId {
        let id = self.states.len();
        self.states.push(State { is_accepting });
        self.transitions.push(Vec::new());
        id
    }

    /// 添加状态转移规则
    pub fn add_transition(
        &mut self,
        from: StateId,
        to: StateId,
        condition: TransitionCondition,
    ) -> Result<(), AddTransitionError> {
        if from >= self.states.len() {
            return Err(AddTransitionError::InvalidFromStateId(
                InvalidStateIdError { state_id: from },
            ));
        }
        if to >= self.states.len() {
            return Err(AddTransitionError::InvalidToStateId(InvalidStateIdError {
                state_id: to,
            }));
        }
        self.transitions[from].push(Transition { to, condition });
        Ok(())
    }

    /// 处理事件，进行状态转移
    pub fn process_event(&mut self, event: Event) -> Option<ProcessEventError> {
        if self.current_state >= self.transitions.len() {
            return Some(ProcessEventError::InvalidStateId);
        }
        let transitions = &self.transitions[self.current_state];
        for transition in transitions {
            if (transition.condition)(event) {
                self.current_state = transition.to;
                return None;
            }
        }
        self.reset();
        Some(ProcessEventError::NoMatchingTransition)
    }

    /// 重置状态机到初始状态
    pub fn reset(&mut self) {
        self.current_state = 0;
    }

    /// 获取当前状态ID
    pub fn get_current_state(&self) -> StateId {
        self.current_state
    }

    /// 检查当前状态是否为接受状态
    pub fn is_in_accepting_state(&self) -> bool {
        self.current_state < self.states.len() && self.states[self.current_state].is_accepting
    }

    /// 获取令牌类型
    pub fn get_token_type(&self) -> T {
        self.token.clone() // 利用Clone约束复制令牌
    }
}

impl<T: fmt::Debug> fmt::Debug for Machine<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Machine")
            .field("current_state", &self.current_state)
            .field("token", &self.token)
            .field("state_count", &self.states.len())
            .finish()
    }
}
