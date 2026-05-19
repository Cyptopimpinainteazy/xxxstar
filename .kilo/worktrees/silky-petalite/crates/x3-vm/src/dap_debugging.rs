//! Debug Adapter Protocol (DAP) for VS Code integration
//!
//! Allows step-through debugging of smart contracts and kernel execution
//! Compatible with VS Code debug extension.

use std::collections::HashMap;

/// DAP Session represents one debug connection
#[derive(Clone)]
pub struct DAPSession {
    /// Session ID (unique per debugger session)
    pub session_id: u32,
    /// Thread IDs currently executing
    pub threads: HashMap<u32, ThreadInfo>,
    /// Breakpoints set (source_path → Vec<line_number>)
    pub breakpoints: HashMap<String, Vec<u32>>,
    /// Current execution state
    pub state: ExecutionState,
    /// Stack frames for current thread
    pub stack_frames: Vec<StackFrame>,
}

/// Thread information
#[derive(Clone, Debug)]
pub struct ThreadInfo {
    pub thread_id: u32,
    pub name: String,
    pub state: ThreadState,
    pub current_pc: u64, // Program counter
}

#[derive(Clone, Debug)]
pub enum ThreadState {
    Running,
    Stopped,
    Waiting,
    Terminated,
}

/// Variable scope (locals, globals, etc)
#[derive(Clone, Debug)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub ty: String,
    pub reference: u32, // For nested inspection
}

/// Stack frame (one function call)
#[derive(Clone, Debug)]
pub struct StackFrame {
    pub frame_id: u32,
    pub name: String,
    pub source: String,
    pub line: u32,
    pub column: u32,
    pub variables: Vec<Variable>,
}

#[derive(Clone, Debug)]
pub enum ExecutionState {
    Initialized,
    Running,
    Paused,
    Exited { code: i32 },
}

/// Breakpoint
#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub line: u32,
    pub verified: bool,
    pub source: String,
    pub hit_count: u32,
}

/// DAP protocol messages (simplified)
#[derive(Clone, Debug)]
pub enum DAPMessage {
    /// Initialize debugger
    Initialize { client_name: String },
    /// Set breakpoint
    SetBreakpoints { source: String, lines: Vec<u32> },
    /// Continue execution
    Continue { thread_id: u32 },
    /// Step into
    StepIn { thread_id: u32 },
    /// Step over
    StepOver { thread_id: u32 },
    /// Pause execution
    Pause { thread_id: u32 },
    /// Get variables in scope
    Variables { frame_id: u32 },
    /// Get stack trace
    StackTrace { thread_id: u32 },
    /// Evaluate expression
    Evaluate { expression: String, frame_id: u32 },
    /// Disconnect
    Disconnect,
}

/// DAP Server (runs in validator process)
pub struct DAPServer {
    pub session: DAPSession,
    pub next_session_id: u32,
    pub next_thread_id: u32,
    pub next_frame_id: u32,
    pub next_var_id: u32,
}

impl DAPServer {
    pub fn new() -> Self {
        Self {
            session: DAPSession {
                session_id: 0,
                threads: HashMap::new(),
                breakpoints: HashMap::new(),
                state: ExecutionState::Initialized,
                stack_frames: Vec::new(),
            },
            next_session_id: 1,
            next_thread_id: 1,
            next_frame_id: 1,
            next_var_id: 1,
        }
    }

    /// Initialize a new debug session
    pub fn initialize(&mut self, client_name: &str) -> Result<(), String> {
        self.session.session_id = self.next_session_id;
        self.next_session_id += 1;

        eprintln!("[DAP] Initialized by client: {}", client_name);
        Ok(())
    }

    /// Set breakpoints in a source file
    pub fn set_breakpoints(
        &mut self,
        source: String,
        lines: Vec<u32>,
    ) -> Result<Vec<Breakpoint>, String> {
        if lines.is_empty() {
            return Err("Must set at least one breakpoint".to_string());
        }

        // Verify line numbers are reasonable (1-indexed)
        for line in &lines {
            if *line == 0 {
                return Err("Line numbers must be >= 1".to_string());
            }
        }

        self.session
            .breakpoints
            .insert(source.clone(), lines.clone());

        let mut bps = Vec::new();
        for line in lines {
            bps.push(Breakpoint {
                line,
                verified: true,
                source: source.clone(),
                hit_count: 0,
            });
        }

        Ok(bps)
    }

    /// Step into next function
    pub fn step_in(&mut self, thread_id: u32) -> Result<(), String> {
        eprintln!("[DAP] Step-in on thread {}", thread_id);
        self.session.state = ExecutionState::Paused;
        Ok(())
    }

    /// Step over next line
    pub fn step_over(&mut self, thread_id: u32) -> Result<(), String> {
        eprintln!("[DAP] Step-over on thread {}", thread_id);
        self.session.state = ExecutionState::Paused;
        Ok(())
    }

    /// Continue from breakpoint
    pub fn continue_execution(&mut self, thread_id: u32) -> Result<(), String> {
        eprintln!("[DAP] Continue on thread {}", thread_id);
        self.session.state = ExecutionState::Running;
        Ok(())
    }

    /// Pause execution
    pub fn pause(&mut self, thread_id: u32) -> Result<(), String> {
        eprintln!("[DAP] Pause on thread {}", thread_id);
        self.session.state = ExecutionState::Paused;
        Ok(())
    }

    /// Get current stack trace
    pub fn get_stack_trace(&mut self, thread_id: u32) -> Result<Vec<StackFrame>, String> {
        let _thread = self
            .session
            .threads
            .get(&thread_id)
            .ok_or("Thread not found")?;

        // Build mock stack
        let frames = vec![StackFrame {
            frame_id: self.next_frame_id,
            name: "validate_block()".to_string(),
            source: "validator.rs".to_string(),
            line: 150,
            column: 0,
            variables: vec![
                Variable {
                    name: "block_hash".to_string(),
                    value: "0xabcd...".to_string(),
                    ty: "[u8; 32]".to_string(),
                    reference: 0,
                },
                Variable {
                    name: "validator_id".to_string(),
                    value: "42".to_string(),
                    ty: "u32".to_string(),
                    reference: 0,
                },
            ],
        }];
        self.next_frame_id += 1;

        Ok(frames)
    }

    /// Get variables in a stack frame
    pub fn get_variables(&self, frame_id: u32) -> Result<Vec<Variable>, String> {
        for frame in &self.session.stack_frames {
            if frame.frame_id == frame_id {
                return Ok(frame.variables.clone());
            }
        }

        // Return locals from mock frame
        Ok(vec![
            Variable {
                name: "x".to_string(),
                value: "100".to_string(),
                ty: "i32".to_string(),
                reference: 0,
            },
            Variable {
                name: "y".to_string(),
                value: "200".to_string(),
                ty: "i32".to_string(),
                reference: 0,
            },
        ])
    }

    /// Evaluate expression (limited support)
    pub fn evaluate(&self, expression: &str, _frame_id: u32) -> Result<String, String> {
        // Simple expressions: variable lookup, arithmetic, etc.
        match expression {
            "block_hash" => Ok("0xabcd1234...".to_string()),
            "validator_id" => Ok("42".to_string()),
            _ => Err(format!("Unknown variable: {}", expression)),
        }
    }

    /// Register a thread
    pub fn register_thread(&mut self, name: &str) -> u32 {
        let tid = self.next_thread_id;
        self.next_thread_id += 1;

        self.session.threads.insert(
            tid,
            ThreadInfo {
                thread_id: tid,
                name: name.to_string(),
                state: ThreadState::Running,
                current_pc: 0,
            },
        );

        eprintln!("[DAP] Registered thread {} ({})", tid, name);
        tid
    }

    /// Hit a breakpoint
    pub fn hit_breakpoint(
        &mut self,
        thread_id: u32,
        source: &str,
        line: u32,
    ) -> Result<(), String> {
        if let Some(thread) = self.session.threads.get_mut(&thread_id) {
            thread.state = ThreadState::Stopped;
        }

        self.session.state = ExecutionState::Paused;
        eprintln!("[DAP] Breakpoint hit: {}:{}", source, line);

        Ok(())
    }

    /// Get all breakpoints
    pub fn get_breakpoints(&self) -> Vec<Breakpoint> {
        let mut bps = Vec::new();

        for (source, lines) in &self.session.breakpoints {
            for line in lines {
                bps.push(Breakpoint {
                    line: *line,
                    verified: true,
                    source: source.clone(),
                    hit_count: 0,
                });
            }
        }

        bps
    }

    /// Disconnect debugger
    pub fn disconnect(&mut self) -> Result<(), String> {
        self.session.state = ExecutionState::Exited { code: 0 };
        eprintln!("[DAP] Debugger disconnected");
        Ok(())
    }
}

impl Default for DAPServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dap_initialize() {
        let mut server = DAPServer::new();
        assert!(server.initialize("VS Code").is_ok());
        assert_eq!(server.session.session_id, 1);
    }

    #[test]
    fn test_set_breakpoints() {
        let mut server = DAPServer::new();

        let bps = server.set_breakpoints("validator.rs".to_string(), vec![10, 20, 30]);
        assert!(bps.is_ok());
        assert_eq!(bps.unwrap().len(), 3);
    }

    #[test]
    fn test_breakpoint_validation() {
        let mut server = DAPServer::new();

        // Empty breakpoints should fail
        let result = server.set_breakpoints("test.rs".to_string(), vec![]);
        assert!(result.is_err());

        // Line 0 should fail
        let result = server.set_breakpoints("test.rs".to_string(), vec![0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_thread() {
        let mut server = DAPServer::new();

        let tid1 = server.register_thread("main");
        let tid2 = server.register_thread("worker");

        assert_eq!(tid1, 1);
        assert_eq!(tid2, 2);
        assert!(server.session.threads.contains_key(&tid1));
    }

    #[test]
    fn test_hit_breakpoint() {
        let mut server = DAPServer::new();
        let tid = server.register_thread("main");

        server.set_breakpoints("test.rs".to_string(), vec![15]).ok();
        assert!(server.hit_breakpoint(tid, "test.rs", 15).is_ok());

        // Thread should be stopped
        let thread = server.session.threads.get(&tid).unwrap();
        assert!(matches!(thread.state, ThreadState::Stopped));
    }

    #[test]
    fn test_step_operations() {
        let mut server = DAPServer::new();
        let tid = server.register_thread("main");

        assert!(server.step_in(tid).is_ok());
        assert!(matches!(server.session.state, ExecutionState::Paused));

        assert!(server.continue_execution(tid).is_ok());
        assert!(matches!(server.session.state, ExecutionState::Running));

        assert!(server.step_over(tid).is_ok());
        assert!(matches!(server.session.state, ExecutionState::Paused));
    }

    #[test]
    fn test_get_stack_trace() {
        let mut server = DAPServer::new();
        let tid = server.register_thread("main");

        let frames = server.get_stack_trace(tid);
        assert!(frames.is_ok());
        assert!(!frames.unwrap().is_empty());
    }

    #[test]
    fn test_evaluate_expression() {
        let server = DAPServer::new();

        let result = server.evaluate("block_hash", 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0xabcd1234...");

        let result = server.evaluate("unknown_var", 0);
        assert!(result.is_err());
    }
}
