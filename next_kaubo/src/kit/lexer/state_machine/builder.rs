use std::fmt;

use super::common::Machine;
use super::error::BuildMachineError;

pub fn build_keyword_machine<T>(keyword: &str, token: T) -> Result<Machine<T>, BuildMachineError>
where
    T: fmt::Debug + Clone + 'static,
{
    let mut machine = Machine::new(token);
    let mut current_state = machine.get_current_state();
    for (i, c) in keyword.chars().enumerate() {
        let is_accepting = i == keyword.len() - 1;
        let next_state = machine.add_state(is_accepting);
        machine.add_transition(current_state, next_state, Box::new(move |event| event == c))?;
        current_state = next_state;
    }

    Ok(machine)
}

pub fn build_string_machine<T>(token_string: T) -> Result<Machine<T>, BuildMachineError>
where
    T: fmt::Debug + Clone + 'static,
{
    let mut machine = Machine::new(token_string);
    let s0 = machine.get_current_state();
    let s1 = machine.add_state(false);
    let s2 = machine.add_state(true);
    let s3 = machine.add_state(false);
    let s4 = machine.add_state(true);

    machine
        .add_transition(s0, s1, Box::new(|c| c == '"'))
        .unwrap();
    machine
        .add_transition(s1, s2, Box::new(|c| c == '"'))
        .unwrap();
    machine
        .add_transition(s1, s1, Box::new(|c| c != '"'))
        .unwrap();
    machine
        .add_transition(s0, s3, Box::new(|c| c == '\''))
        .unwrap();
    machine
        .add_transition(s3, s4, Box::new(|c| c == '\''))
        .unwrap();
    machine
        .add_transition(s3, s3, Box::new(|c| c != '\''))
        .unwrap();

    Ok(machine)
}
