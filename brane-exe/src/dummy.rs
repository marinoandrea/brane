//  DUMMY.rs
//    by Lut99
// 
//  Created:
//    13 Sep 2022, 16:43:11
//  Last edited:
//    09 Jan 2023, 15:56:29
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements a Dummy virtual machine for unit test purposes only.
// 

use std::collections::HashMap;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use async_trait::async_trait;
use log::info;

use brane_ast::Workflow;
use brane_ast::locations::Location;
use brane_ast::ast::{DataName, Edge, SymTable};
use specifications::data::{AccessKind, AvailabilityKind};

pub use crate::errors::VmError as Error;
use crate::spec::{CustomGlobalState, RunState, TaskInfo, VmPlugin};
use crate::value::FullValue;
use crate::vm::Vm;


/***** LIBRARY *****/
/// Defines the global, shared state for the DummyVm.
#[derive(Clone, Debug)]
pub struct DummyState {
    /// The text to buffer when writing to stdout.
    /// 
    /// It looks overkill to have a mutex here, but this is required in the test of `thread.rs` due to it not using a wrapping VM.
    pub text : Arc<Mutex<String>>,
}

impl CustomGlobalState for DummyState {}



/// The DummyPlugin implements the missing functions for the Dummy VM. As the name implies, these don't do any actual work.
pub struct DummyPlugin;

#[async_trait::async_trait]
impl VmPlugin for DummyPlugin {
    type GlobalState = DummyState;
    type LocalState  = ();

    type PreprocessError = Error;
    type ExecuteError    = Error;
    type StdoutError     = Error;
    type CommitError     = Error;


    async fn preprocess(_global: Arc<RwLock<Self::GlobalState>>, _local: Self::LocalState, _loc: Location, name: DataName, _preprocess: specifications::data::PreprocessKind) -> Result<AccessKind, Self::PreprocessError> {
        info!("Processing dummy `DummyVm::preprocess()` call for intermediate result '{}'", name);

        // We also accept it with a dummy accesskind
        Ok(AccessKind::File{ path: PathBuf::new() })
    }

    async fn execute(_global: &Arc<RwLock<Self::GlobalState>>, _local: &Self::LocalState, info: TaskInfo<'_>) -> Result<Option<FullValue>, Self::ExecuteError> {
        info!("Processing dummy call to '{}'@'{}' with {} in {}[{}]...",
            info.name,
            info.location,
            info.args.iter().map(|(n, v)| format!("{}={:?}", n, v)).collect::<Vec<String>>().join(","),
            info.package_name,
            info.package_version,
        );

        // Return according to the name of the function called
        match info.name {
            "hello_world"   => Ok(Some(FullValue::String("Hello, world!".into()))),
            "run_script"    => Ok(Some(FullValue::Void)),
            "aggregate"     => Ok(Some(FullValue::Void)),
            "local_compute" => Ok(Some(FullValue::Void)),
            _               => Ok(None),
        }
    }

    async fn stdout(global: &Arc<RwLock<Self::GlobalState>>, _local: &Self::LocalState, text: &str, newline: bool) -> Result<(), Self::StdoutError> {
        info!("Processing dummy stdout write (newline: {})...",
            if newline { "yes" } else { "no" },
        );

        // Get the global state and append the text
        let state     : RwLockWriteGuard<DummyState> = global.write().unwrap();
        let mut stext : MutexGuard<String>           = state.text.lock().unwrap();
        stext.push_str(&format!("{}{}", text, if newline { "\n" } else { "" }));

        // Done
        Ok(())
    }

    async fn publicize(_global: &Arc<RwLock<Self::GlobalState>>, _local: &Self::LocalState, _loc: &Location, name: &str, path: &Path) -> Result<(), Self::CommitError> {
        info!("Processing dummy publicize for result '{}' @ '{:?}'...",
            name, path.display(),
        );

        // We don't really do anything, unfortunately
        Ok(())
    }
    async fn commit(_global: &Arc<RwLock<Self::GlobalState>>, _local: &Self::LocalState, _loc: &Location, name: &str, path: &Path, data_name: &str) -> Result<(), Self::CommitError> {
        info!("Processing dummy commit for result '{}' @ '{:?}' to '{}'...",
            name, path.display(), data_name,
        );

        // We don't really do anything, unfortunately
        Ok(())
    }
}



/// Defines a Dummy planner that simply assigns 'localhost' to every task it can find.
pub struct DummyPlanner {}

impl DummyPlanner {
    /// Helper function that plans the given list of edges for the dummy VM.
    /// 
    /// This function cannot fail, since it just basically plans anything to have the AST be in a valid state.
    /// 
    /// # Arguments
    /// - `table`: The SymbolTable where this edge lives in.
    /// - `edges`: The given list to plan.
    /// 
    /// # Returns
    /// Nothing, but does change the given list.
    fn plan_edges(table: &mut SymTable, edges: &mut Vec<Edge>) {
        for e in edges {
            if let Edge::Node{ at, input, result, .. } = e {
                // We simply assign all locations to localhost
                *at = Some("localhost".into());

                // For all dataset/intermediate result inputs, we assert they are available on the local location
                for (name, avail) in input {
                    // Just set it as available to _something_, for testing purposes.
                    *avail = Some(AvailabilityKind::Available { how: AccessKind::File{ path: PathBuf::from(name.name()) } });
                }

                // Then, we make the intermediate result available at the location where the function is being run (if there is any)
                if let Some(name) = result {
                    // Insert an entry in the list detailling where to access it and how
                    table.results.insert(name.clone(), "localhost".into());
                }
            }
        }

        // Done
    }



    /// Plans the given workflow by assigning `localhost` to every task it can find.
    /// 
    /// # Arguments
    /// - `workflow`: The Workflow to plan.
    /// 
    /// # Returns
    /// The same workflow, but now with planned locations.
    /// 
    /// # Panics
    /// This function panics if the workflow was malformed somehow.
    pub fn plan(workflow: Workflow) -> Workflow {
        let mut workflow: Workflow = workflow;

        // Get the symbol table muteable, so we can... mutate... it
        let mut table: Arc<SymTable> = Arc::new(SymTable::new());
        mem::swap(&mut workflow.table, &mut table);
        let mut table: SymTable      = Arc::try_unwrap(table).unwrap();

        // Do the main edges first
        {
            // Start by getting a list of all the edges
            let mut edges: Arc<Vec<Edge>> = Arc::new(vec![]);
            mem::swap(&mut workflow.graph, &mut edges);
            let mut edges: Vec<Edge>      = Arc::try_unwrap(edges).unwrap();

            // Plan them
            Self::plan_edges(&mut table, &mut edges);

            // Move the edges back
            let mut edges: Arc<Vec<Edge>> = Arc::new(edges);
            mem::swap(&mut edges, &mut workflow.graph);
        }

        // Then we do the function edges
        {
            // Start by getting the map
            let mut funcs: Arc<HashMap<usize, Vec<Edge>>> = Arc::new(HashMap::new());
            mem::swap(&mut workflow.funcs, &mut funcs);
            let mut funcs: HashMap<usize, Vec<Edge>>      = Arc::try_unwrap(funcs).unwrap();

            // Iterate through all of the edges
            for edges in funcs.values_mut() {
                Self::plan_edges(&mut table, edges);
            }

            // Put the map back
            let mut funcs: Arc<HashMap<usize, Vec<Edge>>> = Arc::new(funcs);
            mem::swap(&mut funcs, &mut workflow.funcs);
        }

        // Then, put the table back
        let mut table: Arc<SymTable> = Arc::new(table);
        mem::swap(&mut table, &mut workflow.table);

        // Done
        workflow
    }
}



/// Defines a Dummy VM that may be used to test.
pub struct DummyVm {
    /// The internal state of the VM in between runs.
    state : RunState<DummyState>,
}

impl DummyVm {
    /// Constructor for the DummyVm that initializes it to an never-used-before-but-ready-for-everything VM.
    /// 
    /// # Returns
    /// A new instance of a DummyVm.
    #[inline]
    pub fn new() -> Self {
        Self {
            state : Self::new_state(DummyState{ text: Arc::new(Mutex::new(String::new())) }),
        }
    }



    /// Prints the buffered text, clearing it again.
    /// 
    /// # Returns
    /// Nothing, but does print to stdout.
    pub fn flush_stdout(&self) {
        // Fetch the global state if there is one
        let state    : RwLockWriteGuard<DummyState> = self.state.global.write().unwrap();
        let mut text : MutexGuard<String>           = state.text.lock().unwrap();
        print!("{}", text);
        *text = String::new();
    }
}

impl Default for DummyVm {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Vm for DummyVm {
    type GlobalState = DummyState;
    type LocalState  = ();



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
    #[inline]
    fn store_state(this: &Arc<RwLock<Self>>, state: RunState<Self::GlobalState>) -> Result<(), Error> {
        // Get a lock and store it
        let mut lock: RwLockWriteGuard<Self> = this.write().unwrap();
        lock.state = state;
        Ok(())
    }

    /// A function that returns the VM's runtime state in the parent struct.
    /// 
    /// This is important and will be used later.
    /// 
    /// # Returns
    /// The RunState of this application if it exists, or else None.
    /// 
    /// # Errors
    /// This function may error for its own reasons.
    #[inline]
    fn load_state(this: &Arc<RwLock<Self>>) -> Result<RunState<Self::GlobalState>, Error> {
        // Get a lock and read it
        let lock: RwLockReadGuard<Self> = this.read().unwrap();
        Ok(lock.state.clone())
    }
}
