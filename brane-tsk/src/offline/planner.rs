//  PLANNER.rs
//    by Lut99
// 
//  Created:
//    24 Oct 2022, 16:40:21
//  Last edited:
//    14 Nov 2022, 10:54:39
//  Auto updated?
//    Yes
// 
//  Description:
//!   A very trivial planner, that simple plans every dataset to run on
//!   'localhost'.
// 

use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;

use log::debug;

use brane_ast::Workflow;
use brane_ast::ast::{DataName, Edge, SymTable};
use specifications::data::{AccessKind, AvailabilityKind, DataIndex};

use crate::errors::PlanError;
use crate::spec::{self, LOCALHOST};


/***** HELPER FUNCTIONS *****/
/// Helper function that plans the given list of edges.
/// 
/// # Arguments
/// - `table`: The SymbolTable where this edge lives in.
/// - `edges`: The given list to plan.
/// - `dindex`: The DataIndex we use to resolve data references.
/// 
/// # Returns
/// Nothing, but does change the given list.
/// 
/// # Errors
/// This function may error if the given list of edges was malformed (usually due to unknown or inaccessible datasets or results).
fn plan_edges(table: &mut SymTable, edges: &mut [Edge], dindex: &Arc<DataIndex>) -> Result<(), PlanError> {
    for (i, e) in edges.iter_mut().enumerate() {
        if let Edge::Node{ task, at, input, result, .. } = e {
            debug!("Planning task '{}' (edge {})...", table.tasks[*task].name(), i);

            // We simply assign all locations to localhost
            *at = Some(LOCALHOST.into());
            debug!("Task '{}' planned at '{}'", table.tasks[*task].name(), LOCALHOST);

            // For all dataset/intermediate result inputs, we assert they are available on the local location
            for (name, avail) in input {
                OfflinePlanner::plan_data(name, avail, dindex, &table.results)?;
            }

            // Then, we make the intermediate result available at the location where the function is being run (if there is any)
            if let Some(name) = result {
                // Insert an entry in the list detailling where to access it and how
                debug!("Making intermediate result '{}' accessible after execution of '{}' on '{}'", name, table.tasks[*task].name(), LOCALHOST);
                table.results.insert(name.clone(), LOCALHOST.into());
            }
        }
    }

    // Done
    debug!("Planning success");
    Ok(())
}





/***** LIBRARY *****/
/// The planner is in charge of assigning locations to tasks in a workflow. This one is very simple, assigning 'localhost' to whatever it sees.
#[derive(Debug)]
pub struct OfflinePlanner {
    /// The local data index to resolve datasets with.
    data_index : Arc<DataIndex>,
}

impl OfflinePlanner {
    /// Constructor for the OfflinePlanner.
    /// 
    /// # Arguments
    /// - `data_index`: The DataIndex that is used to resolve datasets at plantime.
    /// 
    /// # Returns
    /// A new OfflinePlanner instance.
    #[inline]
    pub fn new(data_index: Arc<DataIndex>) -> Self {
        Self {
            data_index,
        }
    }



    /// Plans the given task offline.
    /// 
    /// # Arguments
    /// - `name`: The name of the dataset or intermediate result, as a DataName (so we can distinguish between the two).
    /// - `avail`: The availability for this dataset that we will be updating.
    /// - `dindex`: The DataIndex we use to see what datasets are actually available where.
    /// - `results`: The map of results that are known in this workflow.
    /// 
    /// # Returns
    /// Nothing, but does change the dataset's availability.
    pub fn plan_data(name: &DataName, avail: &mut Option<AvailabilityKind>, dindex: &Arc<DataIndex>, results: &HashMap<String, String>) -> Result<(), PlanError> {
        match name {
            DataName::Data(name) => {
                if let Some(info) = dindex.get(name) {
                    // Check if it is local or remote
                    if let Some(access) = info.access.get(LOCALHOST) {
                        debug!("Input dataset '{}' is locally available", name);
                        *avail = Some(AvailabilityKind::Available { how: access.clone() });
                    } else {
                        // We don't download, so always unavailable
                        return Err(PlanError::DatasetUnavailable{ name: name.clone(), locs: vec![] });
                    }
                } else {
                    return Err(PlanError::UnknownDataset{ name: name.clone() });
                }
            },

            DataName::IntermediateResult(name) => {
                // It has to be declared before
                if let Some(loc) = results.get(name) {
                    // Match on whether it is available locally or not
                    if LOCALHOST == loc {
                        debug!("Input intermediate result '{}' is locally available", name);
                        *avail = Some(AvailabilityKind::Available { how: AccessKind::File{ path: PathBuf::from(name) } });
                    } else {
                         // We don't download, so always unavailable
                         return Err(PlanError::IntermediateResultUnavailable{ name: name.clone(), locs: vec![] });
                    }
                } else {
                    return Err(PlanError::UnknownIntermediateResult{ name: name.clone() });
                }
            },
        }

        // Done
        Ok(())
    }
}

#[async_trait::async_trait]
impl spec::Planner for OfflinePlanner {
    async fn plan(&self, workflow: brane_ast::Workflow) -> Result<Workflow, PlanError> {
        let mut workflow = workflow;

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
            debug!("Planning main edges...");
            plan_edges(&mut table, &mut edges, &self.data_index)?;

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
            for (idx, edges) in &mut funcs {
                debug!("Planning '{}' edges...", table.funcs[*idx].name);
                plan_edges(&mut table, edges, &self.data_index)?;
            }

            // Put the map back
            let mut funcs: Arc<HashMap<usize, Vec<Edge>>> = Arc::new(funcs);
            mem::swap(&mut funcs, &mut workflow.funcs);
        }

        // Then, put the table back
        let mut table: Arc<SymTable> = Arc::new(table);
        mem::swap(&mut table, &mut workflow.table);

        // Done
        Ok(workflow)
    }
}
