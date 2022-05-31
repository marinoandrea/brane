/* TESTS.rs
 *   by Lut99
 *
 * Created:
 *   24 May 2022, 12:50:22
 * Last edited:
 *   24 May 2022, 22:15:52
 * Auto updated?
 *   Yes
 *
 * Description:
 *   File that contains the unit tests for various VM instructions.
**/

use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};

use async_trait::async_trait;
use bytes::BytesMut;
use specifications::common::{FunctionExt, Value};
use specifications::package::PackageIndex;

use crate::bytecode::{ChunkMut, FunctionMut, Opcode};
use crate::executor::{ExecutorError, ServiceState, VmExecutor};
use crate::stack::Slot;
use crate::vm::{Vm, VmError, VmOptions};


/***** ERRORS *****/
/// Defines errors that occur during testing, that are not part of the testing
#[derive(Debug)]
enum TestError {
    /// Could not create a new VM
    VmError{ err: VmError },
}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use TestError::*;
        match self {
            VmError{ err } => write!(f, "Could not create new VM: {}", err),
        }
    }
}

impl Error for TestError {}

impl From<VmError> for TestError {
    #[inline]
    fn from(err: VmError) -> Self {
        Self::VmError{ err }
    }
}





/***** HELPER MACROS *****/
/// Takes a series of opcodes and binary data, and converts them into a BytesMut
macro_rules! into_bytes {
    () => {
        BytesMut::new()
    };

    ($byte:expr) => {
        // Get the byte
        BytesMut::from([ u8::from($byte) ].as_ref())
    };

    ($byte:expr,$($bytes:expr),+) => {
        {
            // Convert the left one into an array
            let mut left = into_bytes!($byte);
            // Convert the rest into an array
            let right = into_bytes!($($bytes),+);
            // Join them
            left.extend_from_slice(&right);
            // Done
            left
        }
    };
}

/// Does the actual stack checking
macro_rules! _check_stack {
    ($vm:ident,$i:expr,$value:expr) => {
        {
            // Cast the value to a Slot
            let slot = Slot::from_value($value, &$vm.globals, &mut $vm.heap).unwrap();

            // Check at that index
            if $i >= $vm.stack.len() { panic!("Stack is not long enough\n{}", $vm.stack); }
            match ($vm.stack.get($i), &slot) {
                // Handle objects through the heap
                (Slot::Object(got), Slot::Object(expected)) => {
                    if got.get() != expected.get() { panic!("Object behind Stack slot {} does not equal {} (got {})", $i, expected, got); }
                },

                // Otherwise, just check
                (got, expected) => if got != expected { panic!("Stack slot {} does not equal {} (got {})", $i, expected, got); },
            }

            // Return the i (but with one offset for the function)
            1 + $i
        }
    };

    ($vm:ident,$i:expr,$value:expr,$($values:expr),+) => {
        {
            // Do the ones to the right first
            _check_stack!($vm,$i + 1,$($values),+);

            // Then, check this one
            _check_stack!($vm,$i,$value);

            // Return the next i (but with one offset for the function)
            1 + $i + 1
        }
    };
}

/// Checks if the stack matches the values given
macro_rules! check_stack {
    ($vm:ident) => {
        // Make sure the stack is empty
        if $vm.stack.len() > 1 { panic!("Expected empty stack, got {}", $vm.stack); }
    };

    ($vm:ident,$($values:expr),+) => {
        // Check the stack
        let i = _check_stack!($vm,1,$($values),+);
        // Check the length
        if $vm.stack.len() != i { panic!("Expected Stack to have length {}, got {}\n{}", i, $vm.stack.len(), $vm.stack); }
    }
}





/***** HELPER FUNCTIONS *****/
/// Initializes an empty VM (without any packages defined).
fn new_vm() -> Result<Vm<TestExecutor>, TestError> {
    Ok(Vm::new_with(TestExecutor{}, Some(PackageIndex::empty()), Some(VmOptions::default()))?)
}



/// Constructs a main function with the given bytecode and constant register and runs it on the given VM
async fn run_main(vm: &mut Vm<TestExecutor>, code: BytesMut, constants: Vec<Value>) {
    // Create the chunk
    let chunk = ChunkMut { code, constants };

    // Create the function
    let main = FunctionMut::main(chunk);

    // Run the function
    if let Err(err) = vm.main(main).await {
        panic!("{}", err);
    }
}

/// Creates an empty VM and runs the given bytecode and constant register on it as the main function. Returns the VM for possible future use.
async fn run_new_main(code: BytesMut, constants: Vec<Value>) -> Vm<TestExecutor> {
    // Create an empty VM
    let mut vm = match new_vm() {
        Ok(vm)   => vm,
        Err(err) => { panic!("{}", err); }  
    };

    // Run it
    run_main(&mut vm, code, constants).await;

    // Done
    vm
}





/***** EXECUTOR *****/
/// A test executor that doesn't really do anything.
#[derive(Clone, Copy, Debug)]
struct TestExecutor {}

#[async_trait]
impl VmExecutor for TestExecutor {
    /// Calls an external function according to the actual Executor implementation.
    /// 
    /// **Arguments**
    ///  * `call`: The external function call to perform
    ///  * `arguments`: Arguments for the function as key/value pairs
    ///  * `location`: The location where the function should be run. Is a high-level location, defined in infra.yml.
    /// 
    /// **Returns**  
    /// The call's return Value on success, or an ExecutorError upon failure.
    async fn call(
        &self,
        call: FunctionExt,
        _arguments: HashMap<String, Value>,
        _location: Option<String>,
    ) -> Result<Value, ExecutorError> {
        // Print the function we're calling
        println!("Performed external call:");
        print!(" > {}(", call.name);
        let mut first = true;
        for param in call.parameters {
            if first { first = false; }
            else { print!(", "); }
            print!("{}: {}", param.name, param.data_type);
        }
        println!(") -> ???");

        // Done
        Ok(Value::Unit)
    }

    /// Writes a debug message to the client TX stream.
    ///
    /// **Arguments**
    ///  * `text`: The text to write.
    /// 
    /// **Returns**  
    /// Nothing if it was successfull, or an ExecutorError otherwise.
    async fn debug(
        &self,
        text: String,
    ) -> Result<(), ExecutorError> {
        println!("[debug] {}", text);
        Ok(())
    }

    /// Writes an error message to the client TX stream.
    ///
    /// **Arguments**
    ///  * `text`: The text to write.
    /// 
    /// **Returns**  
    /// Nothing if it was successfull, or an ExecutorError otherwise.
    async fn stderr(
        &self,
        text: String,
    ) -> Result<(), ExecutorError> {
        println!("[stderr] {}", text);
        Ok(())
    }

    /// Writes a standard/info message to the client TX stream.
    ///
    /// **Arguments**
    ///  * `text`: The text to write.
    /// 
    /// **Returns**  
    /// Nothing if it was successfull, or an ExecutorError otherwise.
    async fn stdout(
        &self,
        text: String,
    ) -> Result<(), ExecutorError> {
        println!("[stdout] {}", text);
        Ok(())
    }

    /* TIM */
    /// **Edited: changed return type to also return ExecutorErrors.**
    ///
    /// Performs an external function call, but blocks until the call has reached the desired state instead of until it is completed.
    /// 
    /// **Arguments**
    ///  * `service`: The service to call.
    ///  * `state`: The state to wait for.
    /// 
    /// **Result**  
    /// Returns nothing if the service was launched successfully and the state reached, or an ExecutorError otherwise.
    async fn wait_until(
        &self,
        _service: String,
        _state: ServiceState,
    ) -> Result<(), ExecutorError> {
        // Do not wait
        Ok(())
    }
}





/***** TESTS *****/
/// Tests the VM in general, i.e., whether we can start it and run an empty function.
#[tokio::test]
async fn test_vm() {
    // Create an empty VM
    let mut vm = match new_vm() {
        Ok(vm)   => vm,
        Err(err) => { panic!("{}", err); }  
    };

    // Run an empty main function
    run_main(&mut vm, into_bytes!(), vec![]).await;

    // Make sure the stack is empty
    check_stack!(vm);

    // Done!
}



/// Test OP_CONSTANT
#[tokio::test]
async fn test_op_constant() {
    // Push an integer
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0
    ), vec![ Value::Integer(42) ]).await;
    check_stack!(vm, Value::Integer(42));

    // Push a float
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0
    ), vec![ Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Real(42.0));

    // Push a boolean (both kinds)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1
    ), vec![ Value::Boolean(true), Value::Boolean(false) ]).await;
    check_stack!(vm, Value::Boolean(true), Value::Boolean(false));

    // Push a string
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0
    ), vec![ Value::Unicode("Hello there!".into()) ]).await;
    check_stack!(vm, Value::Unicode("Hello there!".into()));

    // Done!
}



/// Test OP_ADD
#[tokio::test]
async fn test_op_add() {
    // Run a program adding two integral constants
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::ADD
    ), vec![ Value::Integer(42) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Integer(84));

    // Now run one with floats
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::ADD
    ), vec![ Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(84.0));

    // Now run one with mixed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::ADD
    ), vec![ Value::Integer(42), Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(84.0));

    // Now run one with mixed but reversed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 1,
        Opcode::CONSTANT, 0,
        Opcode::ADD
    ), vec![ Value::Integer(42), Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(84.0));

    // Done!
}

/// Test OP_SUB
#[tokio::test]
async fn test_op_sub() {
    // Run a program adding two constants
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::SUBTRACT
    ), vec![ Value::Integer(84), Value::Integer(42) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Integer(42));

    // Run a program adding two constants but now floats
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::SUBTRACT
    ), vec![ Value::Real(84.0), Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(42.0));

    // Now run one with mixed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::SUBTRACT
    ), vec![ Value::Integer(84), Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(42.0));

    // Now run one with mixed but reversed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 1,
        Opcode::CONSTANT, 0,
        Opcode::SUBTRACT
    ), vec![ Value::Integer(84), Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(-42.0));

    // Done!
}

/// Test OP_MUL
#[tokio::test]
async fn test_op_mul() {
    // Run a program multiplying two integral constants
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::MULTIPLY
    ), vec![ Value::Integer(42) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Integer(1764));

    // Now run one with floats
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::MULTIPLY
    ), vec![ Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(1764.0));

    // Now run one with mixed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::MULTIPLY
    ), vec![ Value::Integer(42), Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(1764.0));

    // Now run one with mixed but reversed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 1,
        Opcode::CONSTANT, 0,
        Opcode::MULTIPLY
    ), vec![ Value::Integer(42), Value::Real(42.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(1764.0));

    // Done!
}

/// Test OP_DIV
#[tokio::test]
async fn test_op_div() {
    // Run a program dividing two constants
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::DIVIDE
    ), vec![ Value::Integer(42), Value::Integer(10) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Integer(4));

    // Run a program adding two constants but now floats
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::DIVIDE
    ), vec![ Value::Real(42.0), Value::Real(10.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(4.2));

    // Now run one with mixed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::DIVIDE
    ), vec![ Value::Integer(42), Value::Real(10.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(4.2));

    // Now run one with mixed but reversed
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 1,
        Opcode::CONSTANT, 0,
        Opcode::DIVIDE
    ), vec![ Value::Integer(42), Value::Real(10.0) ]).await;
    // Make sure the stack is empty
    check_stack!(vm, Value::Real(0.23809523809523808));

    // Done!
}



/// Test OP_EQUAL
#[tokio::test]
async fn test_op_equal() {
    // Run it on A == B (cross)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::EQUAL
    ), vec![ Value::Integer(42), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A == B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::EQUAL
    ), vec![ Value::Integer(42), Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A != B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::EQUAL
    ), vec![ Value::Integer(42), Value::Integer(84) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A == B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::EQUAL
    ), vec![ Value::Real(42.0), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A != B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::EQUAL
    ), vec![ Value::Real(42.0), Value::Real(84.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A == B (string)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::EQUAL
    ), vec![ Value::Unicode("Hello there!".into()), Value::Unicode("Hello there!".into()) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A != B (string)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::EQUAL
    ), vec![ Value::Unicode("Hello there!".into()), Value::Unicode("General Kenobi!".into()) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Done!
}

/// Test OP_LESS
#[tokio::test]
async fn test_op_less() {
    // Run it on A < B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Integer(42), Value::Integer(84) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A > B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Integer(84), Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A == B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::LESS
    ), vec![ Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A < B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Real(42.0), Value::Real(84.0) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A > B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Real(84.0), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A == B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::LESS
    ), vec![ Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A < B (int & float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Integer(42), Value::Real(84.0) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A > B (int & float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Integer(84), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A == B (int & float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Integer(42), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A < B (float & int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Real(42.0), Value::Integer(84) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A > B (float & int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Real(84.0), Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A == B (float & int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::LESS
    ), vec![ Value::Real(42.0), Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Done!
}

/// Test OP_GREATER
#[tokio::test]
async fn test_op_greater() {
    // Run it on A < B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Integer(42), Value::Integer(84) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A > B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Integer(84), Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A == B (int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::GREATER
    ), vec![ Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A < B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Real(42.0), Value::Real(84.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A > B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Real(84.0), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A == B (float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 0,
        Opcode::GREATER
    ), vec![ Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A < B (int & float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Integer(42), Value::Real(84.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A > B (int & float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Integer(84), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A == B (int & float)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Integer(42), Value::Real(42.0) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A < B (float & int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Real(42.0), Value::Integer(84) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Run it on A > B (float & int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Real(84.0), Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(true));

    // Run it on A == B (float & int)
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::CONSTANT, 1,
        Opcode::GREATER
    ), vec![ Value::Real(42.0), Value::Integer(42) ]).await;
    check_stack!(vm, Value::Boolean(false));

    // Done!
}



/// Test an if-statement
#[tokio::test]
async fn test_if_statement() {
    // We run a precompiled code with a 'true' expression
    let offset1: [u8; 2] = 8u16.to_be_bytes();
    let offset2: [u8; 2] = 5u16.to_be_bytes();
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::DEFINE_GLOBAL, 1,
        Opcode::TRUE,
        Opcode::JUMP_IF_FALSE, offset1[0], offset1[1],
        Opcode::POP,
        Opcode::CONSTANT, 2,
        Opcode::SET_GLOBAL, 3,
        Opcode::JUMP, offset2[0], offset2[1],
        Opcode::POP,
        Opcode::CONSTANT, 4,
        Opcode::SET_GLOBAL, 5,
        Opcode::GET_GLOBAL, 1
    ), vec![ Value::Integer(0), Value::Unicode("result".into()), Value::Integer(1), Value::Unicode("result".into()), Value::Integer(2), Value::Unicode("result".into()) ]).await;
    check_stack!(vm, Value::Integer(1));

    // Do the same but with 'false'
    let offset1: [u8; 2] = 8u16.to_be_bytes();
    let offset2: [u8; 2] = 5u16.to_be_bytes();
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::DEFINE_GLOBAL, 1,
        Opcode::FALSE,
        Opcode::JUMP_IF_FALSE, offset1[0], offset1[1],
        Opcode::POP,
        Opcode::CONSTANT, 2,
        Opcode::SET_GLOBAL, 3,
        Opcode::JUMP, offset2[0], offset2[1],
        Opcode::POP,
        Opcode::CONSTANT, 4,
        Opcode::SET_GLOBAL, 5,
        Opcode::GET_GLOBAL, 1
    ), vec![ Value::Integer(0), Value::Unicode("result".into()), Value::Integer(1), Value::Unicode("result".into()), Value::Integer(2), Value::Unicode("result".into()) ]).await;
    check_stack!(vm, Value::Integer(2));
}

/// Test a for-loop
#[tokio::test]
async fn test_for_loop() {
    // We run a precompiled code with a 'true' expression
    let offset1: [u8; 2] = 18u16.to_be_bytes();
    let offset2: [u8; 2] = 26u16.to_be_bytes();
    let mut vm = run_new_main(into_bytes!(
        Opcode::CONSTANT, 0,
        Opcode::DEFINE_GLOBAL, 1,
        Opcode::CONSTANT, 2,
        Opcode::GET_LOCAL, 0,
        Opcode::CONSTANT, 3,
        Opcode::LESS,
        Opcode::JUMP_IF_FALSE, offset1[0], offset1[1],
        Opcode::POP,
        Opcode::GET_GLOBAL, 4,
        Opcode::CONSTANT, 5,
        Opcode::ADD,
        Opcode::SET_GLOBAL, 6,
        Opcode::GET_LOCAL, 0,
        Opcode::CONSTANT, 7,
        Opcode::ADD,
        Opcode::SET_LOCAL, 0,
        Opcode::JUMP_BACK, offset2[0], offset2[1],
        Opcode::POP,
        Opcode::POP,
        Opcode::GET_GLOBAL, 1
    ), vec![ Value::Integer(0), Value::Unicode("res".into()), Value::Integer(0), Value::Integer(10), Value::Unicode("res".into()), Value::Integer(2), Value::Unicode("res".into()), Value::Integer(1) ]).await;
    check_stack!(vm, Value::Integer(20));
}
