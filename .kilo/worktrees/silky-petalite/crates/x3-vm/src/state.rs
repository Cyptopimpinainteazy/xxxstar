//! VM state transition: formal state machine for X3VM execution phases.
//!
//! Tracks the lifecycle of a contract execution: Idle → Executing → Committing →
//! Committed / Reverted. Enforces valid transitions and prevents illegal state changes.

/// States in the X3VM execution lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VmState {
    /// No execution in progress.
    Idle,
    /// Bytecode is being interpreted.
    Executing,
    /// Execution completed successfully; awaiting host commit.
    Committing,
    /// Execution completed and state is committed.
    Committed,
    /// Execution reverted; state is rolled back.
    Reverted,
    /// VM encountered a fatal error (e.g. stack corruption).
    Faulted(String),
}

/// Errors from illegal state transitions.
#[derive(Debug, PartialEq, Eq)]
pub enum StateError {
    /// Transition is not allowed from the current state.
    InvalidTransition {
        from: String,
        to: String,
    },
    /// VM is in a faulted state and cannot be reused.
    VmFaulted(String),
}

/// The state machine controlling X3VM execution phases.
pub struct StateMachine {
    state: VmState,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: VmState::Idle,
        }
    }

    pub fn state(&self) -> &VmState {
        &self.state
    }

    /// Transition Idle → Executing.
    pub fn begin_execution(&mut self) -> Result<(), StateError> {
        self.transition(VmState::Idle, VmState::Executing)
    }

    /// Transition Executing → Committing.
    pub fn signal_success(&mut self) -> Result<(), StateError> {
        self.transition(VmState::Executing, VmState::Committing)
    }

    /// Transition Committing → Committed.
    pub fn finish_commit(&mut self) -> Result<(), StateError> {
        self.transition(VmState::Committing, VmState::Committed)
    }

    /// Transition Executing or Committing → Reverted.
    pub fn revert(&mut self) -> Result<(), StateError> {
        match &self.state {
            VmState::Executing | VmState::Committing => {
                self.state = VmState::Reverted;
                Ok(())
            }
            VmState::Faulted(msg) => Err(StateError::VmFaulted(msg.clone())),
            other => Err(StateError::InvalidTransition {
                from: format!("{other:?}"),
                to: "Reverted".into(),
            }),
        }
    }

    /// Reset to Idle (only from terminal states: Committed or Reverted).
    pub fn reset(&mut self) -> Result<(), StateError> {
        match &self.state {
            VmState::Committed | VmState::Reverted => {
                self.state = VmState::Idle;
                Ok(())
            }
            other => Err(StateError::InvalidTransition {
                from: format!("{other:?}"),
                to: "Idle".into(),
            }),
        }
    }

    fn transition(&mut self, from: VmState, to: VmState) -> Result<(), StateError> {
        if let VmState::Faulted(msg) = &self.state {
            return Err(StateError::VmFaulted(msg.clone()));
        }
        if self.state != from {
            return Err(StateError::InvalidTransition {
                from: format!("{:?}", self.state),
                to: format!("{to:?}"),
            });
        }
        self.state = to;
        Ok(())
    }

    /// Mark the VM as faulted. Cannot be recovered.
    pub fn fault(&mut self, reason: impl Into<String>) {
        self.state = VmState::Faulted(reason.into());
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path() {
        let mut sm = StateMachine::new();
        assert_eq!(sm.state(), &VmState::Idle);
        sm.begin_execution().unwrap();
        assert_eq!(sm.state(), &VmState::Executing);
        sm.signal_success().unwrap();
        assert_eq!(sm.state(), &VmState::Committing);
        sm.finish_commit().unwrap();
        assert_eq!(sm.state(), &VmState::Committed);
        sm.reset().unwrap();
        assert_eq!(sm.state(), &VmState::Idle);
    }

    #[test]
    fn test_revert_from_executing() {
        let mut sm = StateMachine::new();
        sm.begin_execution().unwrap();
        sm.revert().unwrap();
        assert_eq!(sm.state(), &VmState::Reverted);
    }

    #[test]
    fn test_revert_from_committing() {
        let mut sm = StateMachine::new();
        sm.begin_execution().unwrap();
        sm.signal_success().unwrap();
        sm.revert().unwrap();
        assert_eq!(sm.state(), &VmState::Reverted);
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let mut sm = StateMachine::new();
        assert!(sm.signal_success().is_err()); // not Executing
    }

    #[test]
    fn test_fault_blocks_all_transitions() {
        let mut sm = StateMachine::new();
        sm.begin_execution().unwrap();
        sm.fault("stack corrupted");
        assert!(sm.signal_success().is_err());
        assert!(matches!(sm.state(), VmState::Faulted(_)));
    }

    #[test]
    fn test_reset_requires_terminal_state() {
        let mut sm = StateMachine::new();
        assert!(sm.reset().is_err()); // Idle is not terminal
        sm.begin_execution().unwrap();
        assert!(sm.reset().is_err()); // Executing is not terminal
    }

    #[test]
    fn test_revert_then_reset() {
        let mut sm = StateMachine::new();
        sm.begin_execution().unwrap();
        sm.revert().unwrap();
        sm.reset().unwrap();
        assert_eq!(sm.state(), &VmState::Idle);
    }
}
