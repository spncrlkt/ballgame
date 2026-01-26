//! Scenario testing system for deterministic game testing
//!
//! Provides infrastructure for running scripted input tests against
//! headless simulation to verify game mechanics.

pub mod assertions;
pub mod input;
pub mod parser;
pub mod runner;

pub use assertions::{check_sequence, check_state, AssertionError};
pub use input::{ScriptedInputs, TestEntityId};
pub use parser::{EntityDef, ExpectedEvent, FrameInput, InputSnapshot, StateAssertion, TestDefinition, TestExpectations, TestSetup};
pub use runner::{run_test, TestResult};

/// Default path for test scenarios
pub const SCENARIOS_DIR: &str = "tests/scenarios";

/// Default path for test levels
pub const TEST_LEVELS_FILE: &str = "config/test_levels.txt";
