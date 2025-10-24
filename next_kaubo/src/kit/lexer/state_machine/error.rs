use thiserror::Error;

#[derive(Debug, PartialEq)]
pub struct InvalidStateIdError {
    pub state_id: usize,
}

#[derive(Debug, PartialEq)]
pub enum ProcessEventError {
    InvalidStateId,
    NoMatchingTransition,
}

#[derive(Error, Debug, PartialEq)]
pub enum AddTransitionError {
    #[error("Invalid from state ID: {0:?}")]
    InvalidFromStateId(InvalidStateIdError),
    #[error("Invalid to state ID: {0:?}")]
    InvalidToStateId(InvalidStateIdError),
}

#[derive(Error, Debug)]
pub enum BuildMachineError {
    #[error("Add transition error: {0}")]
    MachineMethodError(#[from] AddTransitionError),
    #[error("Empty character sequence")]
    EmptyCharSequence,
}
