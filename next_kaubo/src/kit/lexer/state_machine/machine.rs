use super::error::{AddTransitionError, InvalidStateIdError, ProcessEventError};
use super::types::{Event, TokenKindTrait};

use std::fmt;
pub type StateId = usize;

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
pub struct Machine<TokenKind> {
    pub states: Vec<State>,
    pub transitions: Vec<Vec<Transition>>,
    pub current_state: StateId,
    pub token: TokenKind,
}

impl<TokenKind> Machine<TokenKind>
where
    TokenKind: TokenKindTrait,
{
    pub fn new(token: TokenKind) -> Self {
        let mut states = Vec::new();
        let mut transitions = Vec::new();

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

    pub fn add_state(&mut self, is_accepting: bool) -> StateId {
        let id = self.states.len();
        self.states.push(State { is_accepting });
        self.transitions.push(Vec::new());
        id
    }

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

    pub fn reset(&mut self) {
        self.current_state = 0;
    }

    pub fn get_current_state(&self) -> StateId {
        self.current_state
    }
    pub fn is_in_accepting_state(&self) -> bool {
        self.current_state < self.states.len() && self.states[self.current_state].is_accepting
    }

    pub fn get_token_kind(&self) -> TokenKind {
        self.token.clone()
    }
}

impl<TokenKind: fmt::Debug> fmt::Debug for Machine<TokenKind> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Machine")
            .field("current_state", &self.current_state)
            .field("token", &self.token)
            .field("state_count", &self.states.len())
            .finish()
    }
}
