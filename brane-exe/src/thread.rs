//  THREAD.rs
//    by Lut99
// 
//  Created:
//    09 Sep 2022, 13:23:41
//  Last edited:
//    06 Jan 2023, 11:30:02
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements a single Thread of a VM, which sequentially executes a
//!   given stream of Edges.
// 

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use async_recursion::async_recursion;
use enum_debug::EnumDebug as _;
use futures::future::{BoxFuture, FutureExt};
use log::debug;
use tokio::spawn;
use tokio::task::JoinHandle;

use brane_ast::{DataType, MergeStrategy, Workflow};
use brane_ast::spec::{BuiltinClasses, BuiltinFunctions};
use brane_ast::locations::Location;
use brane_ast::ast::{ClassDef, DataName, Edge, EdgeInstr, FunctionDef, TaskDef};
use specifications::data::{AccessKind, AvailabilityKind};

use crate::dbg_node;
pub use crate::errors::VmError as Error;
use crate::errors::ReturnEdge;
use crate::spec::{CustomGlobalState, CustomLocalState, RunState, TaskInfo, VmPlugin};
use crate::value::{FullValue, Value};
use crate::stack::Stack;
use crate::frame_stack::FrameStack;


/***** TESTS *****/
#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use brane_ast::{compile_program, CompileResult, ParserOptions};
    use brane_ast::traversals::print::ast;
    use brane_shr::utilities::{create_data_index, create_package_index, test_on_dsl_files_async};
    use specifications::data::DataIndex;
    use specifications::package::PackageIndex;
    use super::*;
    use crate::dummy::{DummyPlanner, DummyPlugin, DummyState};


    /// Tests the traversal by generating symbol tables for every file.
    #[tokio::test]
    async fn test_thread() {
        // Setup the simple logger
        #[cfg(feature = "test_logging")]
        if let Err(err) = simplelog::TermLogger::init(log::LevelFilter::Debug, Default::default(), simplelog::TerminalMode::Mixed, simplelog::ColorChoice::Auto) {
            eprintln!("WARNING: Failed to setup logger: {} (no logging for this session)", err);
        }

        // Run the tests on all the files
        test_on_dsl_files_async("BraneScript", |path, code| {
            async move {
                // Start by the name to always know which file this is
                println!("{}", (0..80).map(|_| '-').collect::<String>());
                println!("File '{}' gave us:", path.display());

                // Load the package index
                let pindex: PackageIndex = create_package_index();
                let dindex: DataIndex    = create_data_index();

                // Compile it to a workflow
                let workflow: Workflow = match compile_program(code.as_bytes(), &pindex, &dindex, &ParserOptions::bscript()) {
                    CompileResult::Workflow(wf, warns) => {
                        // Print warnings if any
                        for w in warns {
                            w.prettyprint(path.to_string_lossy(), &code);
                        }
                        wf
                    },
                    CompileResult::Eof(err) => {
                        // Print the error
                        err.prettyprint(path.to_string_lossy(), &code);
                        panic!("Failed to compile to workflow (see output above)");
                    }
                    CompileResult::Err(errs) => {
                        // Print the errors
                        for e in errs {
                            e.prettyprint(path.to_string_lossy(), &code);
                        }
                        panic!("Failed to compile to workflow (see output above)");
                    },
    
                    _ => { unreachable!(); },
                };

                // Run the dummy planner on the workflow
                let workflow: Workflow = DummyPlanner::plan(workflow);

                // Now print the file for prettyness
                let workflow: Workflow = ast::do_traversal(workflow, std::io::stdout()).unwrap();
                println!("{}", (0..40).map(|_| "- ").collect::<String>());

                // Run the program
                let text: Arc<Mutex<String>>     = Arc::new(Mutex::new(String::new()));
                let main: Thread<DummyState, ()> = Thread::new(&workflow, DummyState{ text: text.clone() });
                match main.run::<DummyPlugin>().await {
                    Ok(value) => {
                        println!("Workflow stdout:");
                        print!("{}", text.lock().unwrap());
                        println!();
                        println!("Workflow returned: {:?}", value);
                    },
                    Err(err)  => {
                        err.prettyprint();
                        panic!("Failed to execute workflow (see output above)");
                    },
                }
                println!("{}\n\n", (0..80).map(|_| '-').collect::<String>());
            }
        }).await;
    }
}





/***** HELPER ENUMS *****/
/// Defines the result of an Edge execution.
#[derive(Debug)]
enum EdgeResult {
    /// The Edge completed the thread, returning a (maybe) value.
    Ok(Value),
    /// The Edge execution was a success but the workflow continues (to the given body and the given edge in that body, in fact).
    Pending((usize, usize)),
    /// The Edge execution was a disaster and something went wrong.
    Err(Error),
}





/***** HELPER FUNCTIONS *****/
/// Preprocesses any datasets / intermediate results in the given value.
/// 
/// # Arguments
/// 
/// # Returns
/// Nothing, but does add any preprocessed datasets to `data`.
/// 
/// # Errors
/// This function may error if the given `input` does not contain any of the data in the value _or_ if the referenced input is not yet planned.
#[async_recursion]
#[allow(clippy::too_many_arguments)]
async fn preprocess_value<P: VmPlugin>(global: &Arc<RwLock<P::GlobalState>>, local: &P::LocalState, pc: (usize, usize), task: &TaskDef, at: &Location, value: &FullValue, input: &HashMap<DataName, Option<AvailabilityKind>>, data: &mut HashMap<DataName, JoinHandle<Result<AccessKind, P::PreprocessError>>>) -> Result<(), Error> {
    // If it's a data or intermediate result, get it; skip it otherwise
    let name: DataName = match value {
        // The data and intermediate result, of course
        FullValue::Data(name)               => DataName::Data(name.into()),
        FullValue::IntermediateResult(name) => DataName::IntermediateResult(name.into()),

        // Also handle any nested stuff
        FullValue::Array(values)      => { for v in values { preprocess_value::<P>(global, local, pc, task, at, v, input, data).await?; } return Ok(()); },
        FullValue::Instance(_, props) => { for v in props.values() { preprocess_value::<P>(global, local, pc, task, at, v, input, data).await?; } return Ok(()); },

        // The rest is irrelevant
        _ => { return Ok(()); },
    };

    // Fetch it from the input
    let avail: AvailabilityKind = match input.get(&name) {
        Some(avail) => match avail {
            Some(avail) => avail.clone(),
            None        => { return Err(Error::UnplannedInput{ edge: pc.1, task: task.name().into(), name }); },
        },
        None => { return Err(Error::UnknownInput{ edge: pc.1, task: task.name().into(), name }); },
    };

    // If it is unavailable, download it and make it available
    let access: JoinHandle<_> = match avail {
        AvailabilityKind::Available { how }   => {
            debug!("{} '{}' is locally available", name.variant(), name.name());
            tokio::spawn(async move { Ok(how) })
        },
        AvailabilityKind::Unavailable { how } => {
            debug!("{} '{}' is remotely available", name.variant(), name.name());

            // Call the external transfer function
            // match P::preprocess(global, local, at, &name, how).await {
            //     Ok(access) => access,
            //     Err(err)   => { return Err(Error::Custom{ edge: pc.1, err: Box::new(err) }); }
            // }
            tokio::spawn(P::preprocess(global.clone(), local.clone(), at.clone(), name.clone(), how))
        },
    };

    // Insert it into the map, done
    data.insert(name, access);
    Ok(())
}

/// Runs a single instruction, modifying the given stack and variable register.
/// 
/// # Arguments
/// - `edge`: The index of the edge we're executing (used for debugging purposes).
/// - `idx`: The index of the instruction we're executing (used for debugging purposes).
/// - `instr`: The EdgeInstr to execute.
/// - `stack`: The Stack that represents temporary state for executing.
/// - `fstack`: The FrameStack that we read/write variable from/to.
/// 
/// # Returns
/// The next index to execute. Note that this is _relative_ to the given instruction (so it will typically be 1)
/// 
/// # Errors
/// This function may error if execution of the instruction failed. This is typically due to incorrect runtime typing.
fn exec_instr(edge: usize, idx: usize, instr: &EdgeInstr, stack: &mut Stack, fstack: &mut FrameStack) -> Result<i64, Error> {
    use EdgeInstr::*;
    let next: i64 = match instr {
        Cast{ res_type } => {
            // Get the top value off the stack
            let value: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Any }) },
            };

            // Attempt to cast it based on the value it is
            let value: Value = match value.cast(res_type, fstack.table()) {
                Ok(value) => value,
                Err(err)  => { return Err(Error::CastError{ edge, instr: idx, err }); },
            };

            // Push the value back
            stack.push(value).to_instr(edge, idx)?;
            1
        },
        Pop{} => {
            // Get the top value off the stack and discard it
            if stack.pop().is_none() {
                return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Any });
            };
            1
        },
        PopMarker{} => {
            // Push a pop marker on top of the stack.
            stack.push_pop_marker().to_instr(edge, idx)?;
            1
        },
        DynamicPop{} => {
            // Let the stack handle this one.
            stack.dpop();
            1
        },

        Branch{ next } => {
            // Examine the top value on the the stack
            let value: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Boolean }); }
            };

            // Examine it as a boolean
            let value_type: DataType = value.data_type(fstack.table());
            let value: bool = match value.try_as_bool() {
                Some(value) => value,
                None        => { return Err(Error::StackTypeError { edge, instr: Some(idx), got: value_type, expected: DataType::Boolean }); }
            };

            // Branch only if true
            if value {
                *next
            } else {
                1
            }
        },
        BranchNot{ next } => {
            // Examine the top value on the the stack
            let value: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Boolean }); }
            };

            // Examine it as a boolean
            let value_type: DataType = value.data_type(fstack.table());
            let value: bool = match value.try_as_bool() {
                Some(value) => value,
                None        => { return Err(Error::StackTypeError { edge, instr: Some(idx), got: value_type, expected: DataType::Boolean }); }
            };

            // Branch only if **false**
            if !value {
                *next
            } else {
                1
            }
        },

        Not{} => {
            // Get the top value off the stack
            let value: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            // Get it as a boolean
            let value_type: DataType = value.data_type(fstack.table());
            let value: bool = match value.try_as_bool() {
                Some(value) => value,
                None        => { return Err(Error::StackTypeError { edge, instr: Some(idx), got: value_type, expected: DataType::Boolean }); }
            };

            // Push the negated value back
            stack.push(Value::Boolean { value: !value }).to_instr(edge, idx)?;
            1
        },
        Neg{} => {
            // Get the top value off the stack
            let value: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get it as an integer or real value
            match value {
                Value::Integer { value } => {
                    // Put the negated value back
                    stack.push(Value::Integer { value: -value }).to_instr(edge, idx)?;
                },
                Value::Real { value } => {
                    // Put the negated value back
                    stack.push(Value::Real { value: -value }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                value => { return Err(Error::StackTypeError { edge, instr: Some(idx), got: value.data_type(fstack.table()), expected: DataType::Numeric }); }
            };
            1
        },

        And{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Boolean }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Boolean }) },
            };
            // Get them both as boolean values
            let (lhs_type, rhs_type): (DataType, DataType) = (lhs.data_type(fstack.table()), rhs.data_type(fstack.table()));
            let (lhs, rhs): (bool, bool) = match (lhs.try_as_bool(), rhs.try_as_bool()) {
                (Some(lhs), Some(rhs)) => (lhs, rhs),
                (_, _)                 => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs_type, rhs_type), expected: DataType::Boolean }); }
            };

            // Push the conjunction of the two on top again
            stack.push(Value::Boolean { value: lhs && rhs }).to_instr(edge, idx)?;
            1
        },
        Or{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Boolean }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Boolean }) },
            };
            // Get them both as boolean values
            let (lhs_type, rhs_type): (DataType, DataType) = (lhs.data_type(fstack.table()), rhs.data_type(fstack.table()));
            let (lhs, rhs): (bool, bool) = match (lhs.try_as_bool(), rhs.try_as_bool()) {
                (Some(lhs), Some(rhs)) => (lhs, rhs),
                (_, _)                 => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs_type, rhs_type), expected: DataType::Boolean }); }
            };

            // Push the disjunction of the two on top again
            stack.push(Value::Boolean { value: lhs || rhs }).to_instr(edge, idx)?;
            1
        },

        Add{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Addable }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Addable }) },
            };

            // Get them both as either numeric _or_ string values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the added value back
                    stack.push(Value::Integer { value: lhs + rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the added value back
                    stack.push(Value::Real { value: lhs + rhs }).to_instr(edge, idx)?;
                },
                (Value::String { value: mut lhs }, Value::String { value: rhs }) => {
                    // Put the concatenated value back
                    lhs.push_str(&rhs);
                    stack.push(Value::String { value: lhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Addable }); }
            };
            1
        },
        Sub{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get them both as either numeric _or_ string values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Integer { value: lhs - rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Real { value: lhs - rhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Numeric }); }
            };
            1
        },
        Mul{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get them both as either numeric _or_ string values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the multiplied value back
                    stack.push(Value::Integer { value: lhs * rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the multiplied value back
                    stack.push(Value::Real { value: lhs * rhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Numeric }); }
            };
            1
        },
        Div{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get them both as either numeric _or_ string values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the divided value back
                    stack.push(Value::Integer { value: lhs / rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the divided value back
                    stack.push(Value::Real { value: lhs / rhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Numeric }); }
            };
            1
        },
        Mod{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Integer }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Integer }) },
            };
            // Get them both as integer values
            let (lhs_type, rhs_type): (DataType, DataType) = (lhs.data_type(fstack.table()), rhs.data_type(fstack.table()));
            let (lhs, rhs): (i64, i64) = match (lhs.try_as_int(), rhs.try_as_int()) {
                (Some(lhs), Some(rhs)) => (lhs, rhs),
                (_, _)                 => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs_type, rhs_type), expected: DataType::Integer }); }
            };

            // Push the modulo of the two on top again
            stack.push(Value::Integer { value: lhs % rhs }).to_instr(edge, idx)?;
            1
        },

        Eq{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Simply push if they are the same
            stack.push(Value::Boolean { value: lhs == rhs }).to_instr(edge, idx)?;
            1
        },
        Ne{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Simply push if they are not the same
            stack.push(Value::Boolean { value: lhs != rhs }).to_instr(edge, idx)?;
            1
        },
        Lt{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get them both as numeric values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs < rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs < rhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Numeric }); }
            };
            1
        },
        Le{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get them both as numeric values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs <= rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs <= rhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Numeric }); }
            };
            1
        },
        Gt{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get them both as numeric values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs > rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs > rhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Numeric }); }
            };
            1
        },
        Ge{} => {
            // Pop the lhs and rhs off the stack (reverse order)
            let rhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };
            let lhs: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Numeric }) },
            };

            // Get them both as numeric values
            match (lhs, rhs) {
                (Value::Integer { value: lhs }, Value::Integer { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs >= rhs }).to_instr(edge, idx)?;
                },
                (Value::Real { value: lhs }, Value::Real { value: rhs }) => {
                    // Put the subtracted value back
                    stack.push(Value::Boolean { value: lhs >= rhs }).to_instr(edge, idx)?;
                },

                // Yeah no not that one
                (lhs, rhs) => { return Err(Error::StackLhsRhsTypeError { edge, instr: idx, got: (lhs.data_type(fstack.table()), rhs.data_type(fstack.table())), expected: DataType::Numeric }); }
            };
            1
        },

        Array{ length, res_type } => {
            let mut res_type: DataType = res_type.clone();

            // Pop enough values off the stack
            let mut elems: Vec<Value> = Vec::with_capacity(*length);
            for _ in 0..*length {
                // Pop the value
                let value: Value = match stack.pop() {
                    Some(value) => value,
                    None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: res_type }); }  
                };

                // Update the res_type if necessary; otherwise, make sure this is of the correct type
                if let DataType::Any = &res_type { res_type = value.data_type(fstack.table()); }
                else if res_type != value.data_type(fstack.table()) { return Err(Error::ArrayTypeError { edge, instr: idx, got: value.data_type(fstack.table()), expected: res_type }) }

                // Add the element
                elems.push(value);
            }
            // Remember, stack pushes are in reversed direction
            elems.reverse();

            // Create the array and push it back
            stack.push(Value::Array{ values: elems }).to_instr(edge, idx)?;
            1
        },
        ArrayIndex{ res_type } => {
            // Pop the index
            let index: Value = match stack.pop() {
                Some(index) => index,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Integer }); }
            };
            // as an integer
            let index_type: DataType = index.data_type(fstack.table());
            let index: i64 = match index.try_as_int() {
                Some(index) => index,
                None        => { return Err(Error::StackTypeError { edge, instr: Some(idx), got: index_type, expected: DataType::Integer }); }
            };

            // Get the array itself
            let arr: Value = match stack.pop() {
                Some(arr) => arr,
                None      => { return Err(Error::EmptyStackError{ edge, instr: Some(idx), expected: DataType::Array{ elem_type: Box::new(res_type.clone()) } }); }
            };
            // as an array of values but indexed correctly
            let arr_type: DataType = arr.data_type(fstack.table());
            let mut arr: Vec<Value> = match arr.try_as_array() {
                Some(arr) => arr,
                None      => { return Err(Error::StackTypeError { edge, instr: Some(idx), got: arr_type, expected: DataType::Array{ elem_type: Box::new(res_type.clone()) } }); }
            };

            // Now index the array and push that element back
            if index < 0 || index as usize >= arr.len() { return Err(Error::ArrIdxOutOfBoundsError{ edge, instr: idx, got: index, max: arr.len() }); }

            // Finally, push that element back and return
            stack.push(arr.swap_remove(index as usize)).to_instr(edge, idx)?;
            1
        },
        Instance{ def } => {
            let class: &ClassDef = fstack.table().class(*def);

            // Pop as many elements as are required (wow)
            let mut fields: Vec<Value> = Vec::with_capacity(class.props.len());
            for i in 0..class.props.len() {
                // Pop the value
                let value: Value = match stack.pop() {
                    Some(value) => value,
                    None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: class.props[class.props.len() - 1 - i].data_type.clone() }); }  
                };

                // Make sure this is of the correct type
                if !value.data_type(fstack.table()).allowed_by(&class.props[class.props.len() - 1 - i].data_type) { return Err(Error::InstanceTypeError { edge, instr: idx, got: value.data_type(fstack.table()), class: class.name.clone(), field: class.props[class.props.len() - 1 - i].name.clone(), expected: class.props[class.props.len() - 1 - i].data_type.clone() }) }

                // Add the element
                fields.push(value);
            }
            fields.reverse();

            // Map them with the class names (alphabetically)
            let mut field_names: Vec<std::string::String> = class.props.iter().map(|v| v.name.clone()).collect();
            field_names.sort_by_key(|n| n.to_lowercase());
            let mut values: HashMap<std::string::String, Value> = field_names.into_iter().zip(fields.into_iter()).collect();

            // Push an instance with those values - unless it's a specific builtin
            if class.name == BuiltinClasses::Data.name() {
                stack.push(Value::Data{ name: values.remove("name").unwrap().try_as_string().unwrap() }).to_instr(edge, idx)?;
            } else if class.name == BuiltinClasses::IntermediateResult.name() {
                stack.push(Value::IntermediateResult{ name: values.remove("name").unwrap().try_as_string().unwrap() }).to_instr(edge, idx)?;
            } else {
                stack.push(Value::Instance{ values, def: *def }).to_instr(edge, idx)?;
            }
            1
        },
        Proj{ field } => {
            // Pop the instance value
            let value: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: DataType::Class{ name: format!("withField={}", field) } }); }
            };
            // as an instance
            let value_type: DataType = value.data_type(fstack.table());
            let (mut values, def): (HashMap<std::string::String, Value>, usize) = match value.try_as_instance() {
                Some(value) => value,
                None        => { return Err(Error::StackTypeError { edge, instr: Some(idx), got: value_type, expected: DataType::Class{ name: format!("withField={}", field) } }) }
            };

            // Attempt to find the value with the correct field
            let value: Value = match values.remove(field) {
                Some(value) => value,
                None        => {
                    // Try as function instead
                    let mut res: Option<Value> = None;
                    for m in &fstack.table().class(def).methods {
                        if &fstack.table().func(*m).name == field {
                            res = Some(Value::Method{ values, cdef: def, fdef: *m });
                            break;
                        }
                    }
                    match res {
                        Some(res) => res,
                        None      => { return Err(Error::ProjUnknownFieldError{ edge, instr: idx, class: fstack.table().class(def).name.clone(), field: field.clone() }); }
                    }
                },
            };

            // Push it
            stack.push(value).to_instr(edge, idx)?;
            1
        },

        VarGet{ def } => {
            // Attempt to get the value from the variable register
            let value: Value = match fstack.get(*def) {
                Ok(value) => value.clone(),
                Err(err)  => { return Err(Error::VarGetError{ edge, instr: idx, err }); },
            };

            // Push it
            stack.push(value).to_instr(edge, idx)?;
            1
        },
        VarSet{ def } => {
            // Pop the top value off the stack
            let value: Value = match stack.pop() {
                Some(value) => value,
                None        => { return Err(Error::EmptyStackError { edge, instr: Some(idx), expected: fstack.table().var(*def).data_type.clone() }); }
            };

            // Set it in the register, done
            if let Err(err) = fstack.set(*def, value) { return Err(Error::VarSetError { edge, instr: idx, err }); };
            1
        },

        Null{} => {
            // Push a null
            stack.push(Value::Null).to_instr(edge, idx)?;
            1
        },
        Boolean{ value } => {
            // Push a boolean with the given value
            stack.push(Value::Boolean { value: *value }).to_instr(edge, idx)?;
            1
        },
        Integer{ value } => {
            // Push an integer with the given value
            stack.push(Value::Integer { value: *value }).to_instr(edge, idx)?;
            1
        },
        Real{ value } => {
            // Push a real with the given value
            stack.push(Value::Real { value: *value }).to_instr(edge, idx)?;
            1
        },
        String{ value } => {
            // Push a string with the given value
            stack.push(Value::String { value: value.clone() }).to_instr(edge, idx)?;
            1
        },
        Function{ def } => {
            // Push a function with the given definition
            stack.push(Value::Function { def: *def }).to_instr(edge, idx)?;
            1
        },
    };

    // Done
    Ok(next)
}





/***** LIBRARY *****/
/// Represents a single thread that may be executed.
pub struct Thread<G: CustomGlobalState, L: CustomLocalState> {
    /// The graph containing the main edges to execute (indexed by `usize::MAX`).
    graph : Arc<Vec<Edge>>,
    /// The list of function edges to execute.
    funcs : Arc<HashMap<usize, Vec<Edge>>>,

    /// The 'program counter' of this thread. It first indexed the correct body (`usize::MAX` for main, or else the index of the function), and then the offset within that body.
    pc : (usize, usize),

    /// The stack which we use for temporary values.
    stack  : Stack,
    /// The frame stack is used to process function calls.
    fstack : FrameStack,

    /// The threads that we're blocking on.
    blocking_threads : Vec<(usize, JoinHandle<Result<Value, Error>>)>,

    /// The thread-global custom part of the RunState.
    global : Arc<RwLock<G>>,
    /// The thread-local custom part of the RunState.
    local  : L,
}

impl<G: CustomGlobalState, L: CustomLocalState> Thread<G, L> {
    /// Spawns a new main thread from the given workflow.
    /// 
    /// # Arguments
    /// - `workflow`: The Workflow that this thread will execute.
    /// - `pindex`: The PackageIndex we use to resolve packages.
    /// - `dindex`: The DataIndex we use to resolve datasets.
    /// - `global`: The app-wide custom state with which to initialize this thread.
    /// 
    /// # Returns
    /// A new Thread that may be executed.
    #[inline]
    pub fn new(workflow: &Workflow, global: G) -> Self {
        let global: Arc<RwLock<G>> = Arc::new(RwLock::new(global));
        Self {
            graph : workflow.graph.clone(),
            funcs : workflow.funcs.clone(),

            pc : (usize::MAX, 0),

            stack  : Stack::new(2048),
            fstack : FrameStack::new(512, workflow.table.clone()),

            blocking_threads : vec![],

            global : global.clone(),
            local  : L::new(&global),
        }
    }

    /// Spawns a new main thread that does not start from scratch but instead the given VmState.
    /// 
    /// # Arguments
    /// - `workflow`: The workflow to execute.
    /// - `state`: The runstate to "resume" this thread with.
    #[inline]
    pub fn from_state(workflow: &Workflow, state: RunState<G>) -> Self {
        Self {
            graph : workflow.graph.clone(),
            funcs : workflow.funcs.clone(),

            pc : (usize::MAX, 0),

            stack  : Stack::new(2048),
            fstack : state.fstack,

            blocking_threads : vec![],

            global : state.global.clone(),
            local  : L::new(&state.global),
        }
    }

    /// 'Forks' this thread such that it may branch in a parallel statement.
    /// 
    /// # Arguments
    /// - `offset`: The offset (as a `(body, idx)` pair) where the thread will begin computation in the edges list.
    /// 
    /// # Returns
    /// A new Thread that is partly cloned of this one.
    #[inline]
    pub fn fork(&self, offset: (usize, usize)) -> Self {
        Self {
            graph : self.graph.clone(),
            funcs : self.funcs.clone(),

            pc : offset,

            stack  : Stack::new(2048),
            fstack : self.fstack.fork(),

            blocking_threads : vec![],

            global : self.global.clone(),
            local  : L::new(&self.global),
        }
    }



    /// Saves the important bits of this Thread for a next execution round.
    #[inline]
    fn into_state(self) -> RunState<G> {
        RunState {
            fstack : self.fstack,

            global : self.global,
        }
    }



    /// Executes a single edge, modifying the given stacks and variable register.
    /// 
    /// # Arguments
    /// - `pc`: Points to the current edge to execute (as a `(body, offset)` pair).
    /// - `plugins`: An object implementing various parts of task execution that are dependent on the actual setup (i.e., offline VS instance).
    /// 
    /// # Returns
    /// The next index to execute. Note that this is an _absolute_ index (so it will typically be `idx` + 1)
    /// 
    /// # Errors
    /// This function may error if execution of the edge failed. This is typically due to incorrect runtime typing or due to failure to perform an external function call.
    async fn exec_edge<P: VmPlugin<GlobalState = G, LocalState = L>>(&mut self, pc: (usize, usize)) -> EdgeResult {
        // We can early stop if the program counter is out-of-bounds
        if pc.0 == usize::MAX {
            if pc.1 >= self.graph.len() {
                debug!("Nothing to do (main, PC {} >= #edges {})", pc.1, self.graph.len());
                return EdgeResult::Ok(Value::Void);
            }
        } else {
            let f: &[Edge] = self.funcs.get(&pc.0).unwrap_or_else(|| panic!("Failed to find function with index '{}'", pc.0));
            if pc.1 >= f.len() {
                debug!("Nothing to do ({}, PC {} >= #edges {})", pc.0, pc.1, f.len());
                return EdgeResult::Ok(Value::Void);
            }
        }

        // Get the edge based on the index
        let edge: &Edge = if pc.0 == usize::MAX {
            &self.graph[pc.1]
        } else {
            &self.funcs.get(&pc.0).unwrap_or_else(|| panic!("Failed to find function with index '{}'", pc.0))[pc.1]
        };
        dbg_node!("{}) Executing Edge: {:?}", if pc.0 == usize::MAX { "<Main>" } else { &self.fstack.table().func(pc.0).name }, edge);

        // Match on the specific edge
        use Edge::*;
        let next: (usize, usize) = match edge {
            Node{ task, at, input, result, next, .. } => {
                // Resolve the task
                let task: &TaskDef = self.fstack.table().task(*task);

                // Match the thing to do
                match task {
                    TaskDef::Compute { package, version, function, args_names, requirements } => {
                        debug!("Calling compute task '{}' ('{}' v{})", task.name(), package, version);

                        // Collect the arguments from the stack (remember, reverse order)
                        let mut args: HashMap<String, FullValue> = HashMap::with_capacity(function.args.len());
                        for i in 0..function.args.len() {
                            let i: usize = function.args.len() - 1 - i;

                            // Get the element
                            let value: Value = match self.stack.pop() {
                                Some(value) => value,
                                None        => { return EdgeResult::Err(Error::EmptyStackError{ edge: pc.1, instr: None, expected: function.args[i].clone() }); },
                            };

                            // Check it has the correct type
                            let value_type: DataType = value.data_type(self.fstack.table());
                            if !value_type.allowed_by(&function.args[i]) { return EdgeResult::Err(Error::FunctionTypeError { edge: pc.1, name: task.name().into(), arg: i, got: value_type, expected: function.args[i].clone() }); }

                            // Add it to the list
                            args.insert(args_names[i].clone(), value.into_full(self.fstack.table()));
                        }

                        // Unwrap the location
                        let at: &Location = match at {
                            Some(at) => at,
                            None     => { return EdgeResult::Err(Error::UnresolvedLocation{ edge: pc.1, name: function.name.clone() }); }
                        };

                        // Next, fetch all the datasets required by calling the external transfer function;
                        // The map created maps data names to ways of accessing them locally that may be passed to the container itself.
                        let mut handles: HashMap<DataName, JoinHandle<Result<AccessKind, P::PreprocessError>>> = HashMap::new();
                        for value in args.values() {
                            // Preprocess the given value
                            if let Err(err) = preprocess_value::<P>(&self.global, &self.local, pc, task, at, value, input, &mut handles).await { return EdgeResult::Err(err); };
                        }
                        // Join the handles
                        let mut data: HashMap<DataName, AccessKind> = HashMap::with_capacity(handles.len());
                        for (name, handle) in handles {
                            match handle.await {
                                Ok(res)  => match res {
                                    Ok(access) => { data.insert(name, access); },
                                    Err(err)   => { return EdgeResult::Err(Error::Custom{ edge: pc.1, err: Box::new(err) }); },
                                },
                                Err(err) => { return EdgeResult::Err(Error::Custom{ edge: pc.1, err: Box::new(err) }); },
                            }
                        }

                        // Prepare the TaskInfo for the call
                        let info: TaskInfo = TaskInfo {
                            name            : &function.name,
                            package_name    : package,
                            package_version : version,
                            requirements,

                            args,
                            location : at,
                            input    : data,
                            result,
                        };

                        // Call the external call function with the correct arguments
                        let mut res: Option<Value> = match P::execute(&self.global, &self.local, info).await {
                            Ok(res)  => res.map(|v| v.into_value(self.fstack.table())),
                            Err(err) => { return EdgeResult::Err(Error::Custom{ edge: pc.1, err: Box::new(err) }); },
                        };

                        // If the function returns an intermediate result but returned nothing, that's fine; we inject the result here
                        if function.ret == DataType::IntermediateResult && (res.is_none() || res.as_ref().unwrap() == &Value::Null || res.as_ref().unwrap() == &Value::Void) {
                            // Make the intermediate result available for next steps by possible pushing it to the next registry
                            let name: &str = result.as_ref().unwrap();
                            if let Err(err) = P::publicize(&self.global, &self.local, at, name, &PathBuf::from(name)).await {
                                return EdgeResult::Err(Error::Custom{ edge: pc.1, err: Box::new(err) });
                            }

                            // Return the new, intermediate result
                            res = Some(Value::IntermediateResult{ name: name.into() });
                        }

                        // Verify its return value
                        if let Some(res) = res {
                            // Verification
                            let res_type: DataType = res.data_type(self.fstack.table());
                            if res_type != function.ret { return EdgeResult::Err(Error::ReturnTypeError { edge: pc.1, got: res_type, expected: function.ret.clone() }); }

                            // If we have it anyway, might as well push it onto the stack
                            if let Err(err) = self.stack.push(res) { return EdgeResult::Err(Error::StackError { edge: pc.1, instr: None, err }); }
                        } else if function.ret != DataType::Void { return EdgeResult::Err(Error::ReturnTypeError { edge: pc.1, got: DataType::Void, expected: function.ret.clone() }); }
                    },

                    TaskDef::Transfer {  } => {
                        todo!();
                    },
                }

                // Move to the next edge
                (pc.0, *next)
            },
            Linear{ instrs, next } => {
                // Run the instructions (as long as they don't crash)
                let mut instr_pc: usize = 0;
                while instr_pc < instrs.len() {
                    // It looks a bit funky, but we simply add the relative offset after every constrution to the edge-local program counter
                    instr_pc = (instr_pc as i64 + match exec_instr(pc.1, instr_pc, &instrs[instr_pc], &mut self.stack, &mut self.fstack) {
                        Ok(next) => next,
                        Err(err) => { return EdgeResult::Err(err); },
                    }) as usize;
                }

                // Move to the next edge
                (pc.0, *next)
            },
            Stop{} => {
                // Done no value
                return EdgeResult::Ok(Value::Void);
            },

            Branch{ true_next, false_next, .. } => {
                // Which branch to take depends on the top value of the stack; so get it
                let value: Value = match self.stack.pop() {
                    Some(value) => value,
                    None        => { return EdgeResult::Err(Error::EmptyStackError { edge: pc.1, instr: None, expected: DataType::Boolean }); }  
                };
                // as boolean
                let value_type: DataType = value.data_type(self.fstack.table());
                let value: bool = match value.try_as_bool() {
                    Some(value) => value,
                    None        => { return EdgeResult::Err(Error::StackTypeError { edge: pc.1, instr: None, got: value_type, expected: DataType::Boolean }); }  
                };

                // Branch appropriately
                if value {
                    (pc.0, *true_next)
                } else {
                    match false_next {
                        Some(false_next) => (pc.0, *false_next),
                        None             => { return EdgeResult::Ok(Value::Void); },
                    }
                }
            },
            Parallel{ branches, merge } => {
                // Fork this thread for every branch
                self.blocking_threads.clear();
                self.blocking_threads.reserve(branches.len());
                for (i, b) in branches.iter().enumerate() {
                    // Fork the thread for that branch
                    self.blocking_threads.push((i, spawn(self.fork((pc.0, *b)).run::<P>())));
                }

                // Mark those threads to wait for, and then move to the join
                (pc.0, *merge)
            },
            Join{ merge, next } => {
                // Await the threads first (if any)
                let mut results: Vec<(usize, Value)> = Vec::with_capacity(self.blocking_threads.len());
                for (i, t) in &mut self.blocking_threads {
                    match t.await {
                        Ok(status) => match status {
                            Ok(res)  => { results.push((*i, res)); },
                            Err(err) => { return EdgeResult::Err(err); },
                        },
                        Err(err)   => { return EdgeResult::Err(Error::SpawnError{ edge: pc.1, err }); }
                    }
                }
                self.blocking_threads.clear();

                // Join their values into one according to the merge strategy
                let result: Option<Value> = match merge {
                    MergeStrategy::First | MergeStrategy::FirstBlocking => {
                        if results.is_empty() { panic!("Joining with merge strategy '{:?}' after no threads have been run; this should never happen!", merge); }

                        // It's a bit hard to do this unblocking right now, but from the user the effect will be the same.
                        Some(results.swap_remove(0).1)
                    },
                    MergeStrategy::Last => {
                        if results.is_empty() { panic!("Joining with merge strategy '{:?}' after no threads have been run; this should never happen!", merge); }

                        // It's a bit hard to do this unblocking right now, but from the user the effect will be the same.
                        Some(results.swap_remove(results.len() - 1).1)
                    },

                    MergeStrategy::Sum => {
                        if results.is_empty() { panic!("Joining with merge strategy '{:?}' after no threads have been run; this should never happen!", merge); }

                        // Prepare the sum result
                        let result_type : DataType = results[0].1.data_type(self.fstack.table());
                        let mut result  : Value    = if result_type == DataType::Integer {
                            Value::Integer { value: 0 }
                        } else if result_type == DataType::Real {
                            Value::Real{ value: 0.0 }
                        } else {
                            return EdgeResult::Err(Error::IllegalBranchType { edge: pc.1, branch: 0, merge: *merge, got: result_type, expected: DataType::Numeric });
                        };

                        // Sum the results into that
                        for (i, r) in results {
                            match result {
                                Value::Integer { ref mut value } => {
                                    if let Value::Integer{ value: new_value } = r {
                                        *value += new_value;
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },
                                Value::Real{ ref mut value } => {
                                    if let Value::Real{ value: new_value } = r {
                                        *value += new_value;
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },

                                _ => { unreachable!(); },
                            }
                        }

                        // Done, result is now a combination of all values
                        Some(result)
                    },
                    MergeStrategy::Product => {
                        if results.is_empty() { panic!("Joining with merge strategy '{:?}' after no threads have been run; this should never happen!", merge); }

                        // Prepare the sum result
                        let result_type : DataType = results[0].1.data_type(self.fstack.table());
                        let mut result  : Value    = if result_type == DataType::Integer {
                            Value::Integer { value: 0 }
                        } else if result_type == DataType::Real {
                            Value::Real{ value: 0.0 }
                        } else {
                            return EdgeResult::Err(Error::IllegalBranchType { edge: pc.1, branch: 0, merge: *merge, got: result_type, expected: DataType::Numeric });
                        };

                        // Sum the results into that
                        for (i, r) in results {
                            match result {
                                Value::Integer { ref mut value } => {
                                    if let Value::Integer{ value: new_value } = r {
                                        *value *= new_value;
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },
                                Value::Real{ ref mut value } => {
                                    if let Value::Real{ value: new_value } = r {
                                        *value *= new_value;
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },

                                _ => { unreachable!(); },
                            }
                        }

                        // Done, result is now a combination of all values
                        Some(result)
                    },

                    MergeStrategy::Max => {
                        if results.is_empty() { panic!("Joining with merge strategy '{:?}' after no threads have been run; this should never happen!", merge); }

                        // Prepare the sum result
                        let result_type : DataType = results[0].1.data_type(self.fstack.table());
                        let mut result  : Value    = if result_type == DataType::Integer {
                            Value::Integer { value: i64::MIN }
                        } else if result_type == DataType::Real {
                            Value::Real{ value: f64::NEG_INFINITY }
                        } else {
                            return EdgeResult::Err(Error::IllegalBranchType { edge: pc.1, branch: 0, merge: *merge, got: result_type, expected: DataType::Numeric });
                        };

                        // Sum the results into that
                        for (i, r) in results {
                            match result {
                                Value::Integer { ref mut value } => {
                                    if let Value::Integer{ value: new_value } = r {
                                        if new_value > *value { *value = new_value; }
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },
                                Value::Real{ ref mut value } => {
                                    if let Value::Real{ value: new_value } = r {
                                        if new_value > *value { *value = new_value; }
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },

                                _ => { unreachable!(); },
                            }
                        }

                        // Done, result is now a combination of all values
                        Some(result)
                    },
                    MergeStrategy::Min => {
                        if results.is_empty() { panic!("Joining with merge strategy '{:?}' after no threads have been run; this should never happen!", merge); }

                        // Prepare the sum result
                        let result_type : DataType = results[0].1.data_type(self.fstack.table());
                        let mut result  : Value    = if result_type == DataType::Integer {
                            Value::Integer { value: i64::MAX }
                        } else if result_type == DataType::Real {
                            Value::Real{ value: f64::INFINITY }
                        } else {
                            return EdgeResult::Err(Error::IllegalBranchType { edge: pc.1, branch: 0, merge: *merge, got: result_type, expected: DataType::Numeric });
                        };

                        // Sum the results into that
                        for (i, r) in results {
                            match result {
                                Value::Integer { ref mut value } => {
                                    if let Value::Integer{ value: new_value } = r {
                                        if new_value < *value { *value = new_value; }
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },
                                Value::Real{ ref mut value } => {
                                    if let Value::Real{ value: new_value } = r {
                                        if new_value < *value { *value = new_value; }
                                    } else {
                                        return EdgeResult::Err(Error::BranchTypeError{ edge: pc.1, branch: i, got: r.data_type(self.fstack.table()), expected: result.data_type(self.fstack.table()) });
                                    }
                                },

                                _ => { unreachable!(); },
                            }
                        }

                        // Done, result is now a combination of all values
                        Some(result)
                    },

                    MergeStrategy::All => {
                        if results.is_empty() { panic!("Joining with merge strategy '{:?}' after no threads have been run; this should never happen!", merge); }

                        // Collect them all in an Array of (the same!) values
                        let mut elems     : Vec<Value>       = Vec::with_capacity(results.len());
                        let mut elem_type : Option<DataType> = None;
                        for (i, r) in results {
                            if let Some(elem_type) = &mut elem_type {
                                // Verify it's correctly typed
                                let r_type: DataType = r.data_type(self.fstack.table());
                                if elem_type != &r_type { return EdgeResult::Err(Error::BranchTypeError { edge: pc.1, branch: i, got: r_type, expected: elem_type.clone() }); }

                                // Add it to the list
                                elems.push(r);
                            } else {
                                // It's the first one; make sure there is _something_ and then add it
                                let r_type: DataType = r.data_type(self.fstack.table());
                                if r_type == DataType::Void { return EdgeResult::Err(Error::IllegalBranchType { edge: pc.1, branch: i, merge: *merge, got: DataType::Void, expected: DataType::NonVoid }); }
                                elem_type = Some(r_type);
                                elems.push(r);
                            }
                        }

                        // Set it as an Array result
                        Some(Value::Array{ values: elems })
                    },

                    MergeStrategy::None => None,
                };

                // We can now push that onto the stack, then go to next
                if let Some(result) = result { if let Err(err) = self.stack.push(result) { return EdgeResult::Err(Error::StackError { edge: pc.1, instr: None, err }); } }
                (pc.0, *next)
            },

            Loop{ cond, .. } => {
                // The thing is built in such a way we can just run the condition and be happy
                (pc.0, *cond)
            },

            Call{ next } => {
                // Get the top value off the stack
                let value: Value = match self.stack.pop() {
                    Some(value) => value,
                    None        => { return EdgeResult::Err(Error::EmptyStackError { edge: pc.1, instr: None, expected: DataType::Numeric }) },
                };
                // Get it as a function index
                let def: usize = match value {
                    Value::Function { def }             => def,
                    Value::Method{ values, cdef, fdef } => {
                        // Insert the instance as a stack value, and only then proceed to call
                        let stack_len: usize = self.stack.len();
                        if let Err(err) = self.stack.insert(stack_len - (self.fstack.table().func(fdef).args.len() - 1), Value::Instance{ values, def: cdef }) {
                            return EdgeResult::Err(Error::StackError { edge: pc.1, instr: None, err });
                        };
                        fdef
                    },
                    value                               => { return EdgeResult::Err(Error::StackTypeError { edge: pc.1, instr: None, got: value.data_type(self.fstack.table()), expected: DataType::Callable }); }
                };
                // Resolve the function index
                let sig: &FunctionDef = self.fstack.table().func(def);

                // Double-check the correct values are on the stack
                let stack_len: usize = self.stack.len();
                for (i, v) in self.stack[stack_len - sig.args.len()..].iter().enumerate() {
                    let v_type: DataType = v.data_type(self.fstack.table());
                    if !v_type.allowed_by(&sig.args[i]) { return EdgeResult::Err(Error::FunctionTypeError{ edge: pc.1, name: sig.name.clone(), arg: i, got: v_type, expected: sig.args[i].clone() }); }
                }

                // Either run as a builtin (if it is defined as one) or else run the call
                if sig.name == BuiltinFunctions::Print.name() {
                    // We have one variable that is a string; so print it
                    let text: String = self.stack.pop().unwrap().try_as_string().unwrap();
                    if let Err(err) = P::stdout(&self.global, &self.local, &text, false).await {
                        return EdgeResult::Err(Error::Custom{ edge: pc.1, err: Box::new(err) });
                    }

                    // Done, go to the next immediately
                    (pc.0, *next)

                } else if sig.name == BuiltinFunctions::PrintLn.name() {
                    // We have one variable that is a string; so print it
                    let text: String = self.stack.pop().unwrap().try_as_string().unwrap();
                    if let Err(err) = P::stdout(&self.global, &self.local, &text, true).await {
                        return EdgeResult::Err(Error::Custom{ edge: pc.1, err: Box::new(err) });
                    }

                    // Done, go to the next immediately
                    (pc.0, *next)

                } else if sig.name == BuiltinFunctions::Len.name() {
                    // Fetch the array
                    let array: Vec<Value> = self.stack.pop().unwrap().try_as_array().unwrap();

                    // Push the length back onto the stack
                    if let Err(err) = self.stack.push(Value::Integer { value: array.len() as i64 }) { return EdgeResult::Err(Error::StackError{ edge: pc.1, instr: None, err }); }

                    // We can then go to the next one immediately
                    (pc.0, *next)

                } else if sig.name == BuiltinFunctions::CommitResult.name() {
                    // Fetch the arguments
                    let res_name  : String   = self.stack.pop().unwrap().try_as_intermediate_result().unwrap();
                    let data_name : String   = self.stack.pop().unwrap().try_as_string().unwrap();

                    // Try to find out where this res lives, currently
                    let loc: &String = match self.fstack.table().results().get(&res_name) {
                        Some(loc) => loc,
                        None      => { return EdgeResult::Err(Error::UnknownResult{ edge: pc.1, name: res_name }); },
                    };

                    // Call the external data committer
                    if let Err(err) = P::commit(&self.global, &self.local, loc, &res_name, &PathBuf::from(&res_name), &data_name).await {
                        return EdgeResult::Err(Error::Custom{ edge: pc.1, err: Box::new(err) });
                    };

                    // Push the resulting data onto the stack
                    if let Err(err) = self.stack.push(Value::Data{ name: data_name }) { return EdgeResult::Err(Error::StackError { edge: pc.1, instr: None, err }); }

                    // We can then go to the next one immediately
                    (pc.0, *next)

                } else {
                    // Push the return address onto the frame stack and then go to the correct function
                    if let Err(err) = self.fstack.push(def, (pc.0, *next)) { return EdgeResult::Err(Error::FrameStackPushError{ edge: pc.1, err }); }
                    (def, 0)
                }
            },
            Return{} => {
                // Attempt to pop the top frame off the frame stack
                let (ret, ret_type): ((usize, usize), DataType) = match self.fstack.pop() {
                    Ok(res)  => res,
                    Err(err) => { return EdgeResult::Err(Error::FrameStackPopError { edge: pc.1, err }); }
                };

                // Check if the top value on the stack has this value
                if ret != (usize::MAX, usize::MAX) {
                    // If there is something to return, verify it did
                    if !ret_type.is_void() {
                        // Peek the top value
                        let value: &Value = match self.stack.peek() {
                            Some(value) => value,
                            None        => { return EdgeResult::Err(Error::EmptyStackError { edge: pc.1, instr: None, expected: ret_type }); }
                        };

                        // Compare its data type
                        let value_type: DataType = value.data_type(self.fstack.table());
                        if !value_type.allowed_by(&ret_type) { return EdgeResult::Err(Error::ReturnTypeError{ edge: pc.1, got: value_type, expected: ret_type }); }
                    }

                    // Go to the stack'ed index
                    ret
                } else {
                    // We return the top value on the stack (if any) as a result of this thread
                    return EdgeResult::Ok(self.stack.pop().unwrap_or(Value::Void));
                }
            },
        };

        // Return it
        EdgeResult::Pending(next)
    }



    /// Runs the thread once until it is pending for something (either other threads or external function calls).
    /// 
    /// # Returns
    /// A ThreadStatus that indicates the current position of the thread.
    /// 
    /// # Errors
    /// This function may error if execution of an edge or instruction failed. This is typically due to incorrect runtime typing.
    pub fn run<P: VmPlugin<GlobalState = G, LocalState = L>>(mut self) -> BoxFuture<'static, Result<Value, Error>> {
        async move {
            // Start executing edges from where we left off
            loop {
                // Run the edge
                self.pc = match self.exec_edge::<P>(self.pc).await {
                    EdgeResult::Ok(value)     => { return Ok(value); },
                    EdgeResult::Pending(next) => next,
                    EdgeResult::Err(err)      => { return Err(err); },
                };
            }
        }.boxed()
    }

    /// Runs the thread once until it is pending for something (either other threads or external function calls).
    /// 
    /// This overload supports snippet execution, returning the state that is necessary for the next repl-loop together with the result.
    /// 
    /// # Returns
    /// A tuple of a new VmState and a ThreadStatus that indicates the current position of the thread.
    /// 
    /// # Errors
    /// This function may error if execution of an edge or instruction failed. This is typically due to incorrect runtime typing.
    pub fn run_snippet<P: VmPlugin<GlobalState = G, LocalState = L>>(mut self) -> BoxFuture<'static, Result<(Value, RunState<G>), Error>> {
        async move {
            // Start executing edges from where we left off
            loop {
                // Run the edge
                self.pc = match self.exec_edge::<P>(self.pc).await {
                    // Return not just the value, but also the VmState part of this thread to keep.
                    EdgeResult::Ok(value)     => { return Ok((value, self.into_state())); },
                    EdgeResult::Pending(next) => next,
                    EdgeResult::Err(err)      => { return Err(err); },
                };
            }
        }.boxed()
    }
}
