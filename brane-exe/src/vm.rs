//  VM.rs
//    by Lut99
// 
//  Created:
//    12 Sep 2022, 17:41:33
//  Last edited:
//    23 Dec 2022, 16:37:08
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements the VM trait, which is a simple trait for defining VMs
//!   that use threads.
// 

use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use brane_ast::{SymTable, Workflow};

use crate::errors::VmError;
use crate::spec::{CustomGlobalState, CustomLocalState, RunState, VmPlugin};
use crate::value::FullValue;
use crate::thread::Thread;


/***** TESTS *****/
#[cfg(test)]
pub mod tests {
    use brane_ast::{compile_snippet, CompileResult, ParserOptions};
    use brane_ast::state::CompileState;
    use brane_ast::traversals::print::ast;
    use brane_ast::fetcher::SnippetFetcher;
    use brane_shr::utilities::{create_data_index, create_package_index, test_on_dsl_files_async};
    use specifications::data::DataIndex;
    use specifications::package::PackageIndex;
    use super::*;
    use crate::dummy::{DummyPlanner, DummyPlugin, DummyVm};


    /// Tests the traversal by generating symbol tables for every file.
    #[tokio::test]
    async fn test_snippets() {
        // Setup the simple logger
        #[cfg(feature = "test_logging")]
        if let Err(err) = simplelog::TermLogger::init(log::LevelFilter::Debug, Default::default(), simplelog::TerminalMode::Mixed, simplelog::ColorChoice::Auto) {
            eprintln!("WARNING: Failed to setup logger: {} (no logging for this session)", err);
        }

        // Run the tests on all the files
        test_on_dsl_files_async("BraneScript", |path, code| {
            async move {
                // if !path.to_string_lossy().contains("class.bs") { return; }

                // Start by the name to always know which file this is
                println!("{}", (0..80).map(|_| '-').collect::<String>());
                println!("File '{}' gave us:", path.display());

                // Load the package index
                let pindex: PackageIndex = create_package_index();
                let dindex: DataIndex    = create_data_index();

                // Run the program but now line-by-line (to test the snippet function)
                let mut source: String = String::new();
                let mut state: CompileState = CompileState::new();
                let vm: Arc<RwLock<DummyVm>> = Arc::new(RwLock::new(DummyVm::new()));
                let mut iter = code.split('\n');
                for (offset, l) in SnippetFetcher::new(|| { Ok(iter.next().map(|l| l.into())) }) {
                    // Append the source (for errors only)
                    source.push_str(&l);

                    // Compile the workflow
                    let workflow: Workflow = match compile_snippet(&mut state, l.as_bytes(), &pindex, &dindex, &ParserOptions::bscript()) {
                        CompileResult::Workflow(wf, warns) => {
                            // Print warnings if any
                            for w in warns {
                                w.prettyprint(path.to_string_lossy(), &source);
                            }
                            wf
                        },
                        CompileResult::Eof(err) => {
                            // Fetch more data instead
                            err.prettyprint(path.to_string_lossy(), &source);
                            panic!("Failed to compile to workflow (see output above)");
                        },
                        CompileResult::Err(errs) => {
                            // Print the errors
                            for e in errs {
                                e.prettyprint(path.to_string_lossy(), &source);
                            }
                            panic!("Failed to compile to workflow (see output above)");
                        },
        
                        _ => { unreachable!(); },
                    };

                    // Run the dummy planner on the workflow
                    let workflow: Workflow = DummyPlanner::plan(workflow);

                    // Print the file itself
                    let workflow = ast::do_traversal(workflow, std::io::stdout()).unwrap();
                    println!("{}", (0..40).map(|_| "- ").collect::<String>());

                    // Run the VM on this snippet
                    match DummyVm::run::<DummyPlugin>(vm.clone(), workflow).await {
                        Ok(value) => {
                            println!("Workflow stdout:");
                            vm.read().unwrap().flush_stdout();
                            println!();
                            println!("Workflow returned: {:?}", value);
                        },
                        Err(err)  => {
                            err.prettyprint();
                            panic!("Failed to execute workflow (snippet) (see output above)");
                        },
                    }

                    // Increment the state offset
                    state.offset += offset.line;
                }
                println!("{}\n\n", (0..80).map(|_| '-').collect::<String>());
            }
        }).await;
    }
}





/***** LIBRARY *****/
/// Defines a common interface (and some code) for virtual machines.
#[async_trait]
pub trait Vm {
    /// The type of the thread-global extension to the runtime state.
    type GlobalState : CustomGlobalState;
    /// The type of the thread-local extension to the runtime state.
    type LocalState  : CustomLocalState;



    // Child-specific
    /// A function that stores the given runtime state information in the parent struct.
    /// 
    /// This is important and will be used later.
    /// 
    /// # Arguments
    /// - `state`: The current state of the workflow we have executed.
    /// 
    /// # Returns
    /// Nothing, but should change the internals to return this state later upon request.
    /// 
    /// # Errors
    /// This function may error for its own reasons.
    fn store_state(this: &Arc<RwLock<Self>>, state: RunState<Self::GlobalState>) -> Result<(), VmError>;

    /// A function that returns the VM's runtime state in the parent struct.
    /// 
    /// This is important and will be used later.
    /// 
    /// # Returns
    /// The RunState of this application if it exists, or else None.
    /// 
    /// # Errors
    /// This function may error for its own reasons.
    fn load_state(this: &Arc<RwLock<Self>>) -> Result<RunState<Self::GlobalState>, VmError>;



    // Global
    /// Initializes a new global state based on the given custom part.
    /// 
    /// # Arguments
    /// - `pindex`: The package index which we can use for resolving packages.
    /// - `dindex`: The data index which we can use for resolving datasets.
    /// - `custom`: The custom part of the global state with which we will initialize it.
    /// 
    /// # Returns
    /// A new RunState instance.
    #[inline]
    fn new_state(custom: Self::GlobalState) -> RunState<Self::GlobalState> {
        RunState::new(Arc::new(SymTable::new()), Arc::new(RwLock::new(custom)))
    }

    /// Runs the given workflow, possibly asynchronously (if a parallel is encountered / there are external functions calls and the given closure runs this asynchronously.)
    /// 
    /// # Generic arguments
    /// - `F1`: The closure that performs an external function call for us. See the definition for `ExtCall` to learn how it looks like.
    /// - `F2`: The closure that performs a stdout write to whatever stdout is being used. See the definition for `ExtStdout` to learn how it looks like.
    /// - `F3`: The closure that commits an intermediate results to a full on dataset. This function hides a lot of complexity, and will probably involve contacting the job node to update its datasets in a distributed setting. See the definition for `ExtCommit` to learn how it looks like.
    /// - `E1`: The error type for the `external_call` closure. See the definition of `ExtError` to learn how it looks like.
    /// - `E2`: The error type for the `external_stdout` closure. See the definition of `ExtError` to learn how it looks like.
    /// - `E3`: The error type for the `external_commit` closure. See the definition of `ExtError` to learn how it looks like.
    /// 
    /// # Arguments
    /// - `snippet`: The snippet to compile. This is either the entire workflow, or a snippet of it. In the case of the latter, the internal state will be used (and updated).
    /// - `external_call`: A function that performs the external function call for the poll. It should make use of `.await` if it needs to block the thread somehow.
    /// - `external_stdout`: A function that performs a write to stdout for the poll. It should make use of `.await` if it needs to block the thread somehow.
    /// - `external_commit`: A function that promotes an intermediate result to a fully-fledged, permanent dataset. It should make use of `.await` if it needs to block the thread somehow.
    /// 
    /// # Returns
    /// The result if the Workflow returned any.
    async fn run<P: VmPlugin<GlobalState = Self::GlobalState, LocalState = Self::LocalState>>(this: Arc<RwLock<Self>>, snippet: Workflow) -> Result<FullValue, VmError>
    where
        Self: Sync,
    {
        // Fetch the previous state (if any)
        let mut state: RunState<Self::GlobalState> = Self::load_state(&this)?;
        state.fstack.update_table(snippet.table.clone());

        // Create a new thread with (a copy of) the internal state, if any.
        let main: Thread<Self::GlobalState, Self::LocalState> = Thread::from_state(&snippet, state);

        // Run the workflow
        match main.run_snippet::<P>().await {
            Ok((res, state)) => {
                // Convert the value into a full value (if any)
                let res: FullValue = res.into_full(state.fstack.table());

                // Store the state
                Self::store_state(&this, state)?;

                // Done, return
                Ok(res)
            },
            Err(err) => Err(err),
        }
    }
}
