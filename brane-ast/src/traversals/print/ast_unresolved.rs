//  AST UNRESOLVED.rs
//    by Lut99
// 
//  Created:
//    05 Sep 2022, 11:08:57
//  Last edited:
//    14 Nov 2022, 10:13:04
//  Auto updated?
//    Yes
// 
//  Description:
//!   A print traversal that may print a compiled but unresolved workflow
//!   to stdout.
// 

use std::cell::Ref;
use std::collections::HashSet;

use brane_dsl::DataType;

pub use crate::errors::AstError as Error;
use crate::ast::{Edge, EdgeInstr};
use crate::edgebuffer::{EdgeBuffer, EdgeBufferNode, EdgeBufferNodeLink, EdgeBufferNodePtr};
use crate::ast_unresolved::UnresolvedWorkflow;
use crate::state::{CompileState, FunctionState, TableState, TaskState, VirtualTableState};


/***** MACROS ******/
/// Generates the correct number of spaces for an indent.
macro_rules! indent {
    ($n_spaces:expr) => {
        ((0..$n_spaces).map(|_| ' ').collect::<String>())
    };
}





/***** CONSTANTS *****/
/// Determines the increase in indentation for every nested level.
const INDENT_SIZE: usize = 4;





/***** TRAVERSAL FUNCTIONS *****/
/// Prints the global table of the Workflow.
/// 
/// # Arguments
/// - `table`: The TableState that we print.
/// - `indent`: The indent with which to print the table.
/// 
/// # Returns
/// Nothing, but does print the table to stdout.
pub fn pass_table(table: &TableState, indent: usize) {
    // Simply print all fields in a less-cluttering-to-most-cluttering order

    // Variables first
    for v in &table.vars {
        println!("{}Var {}: {};", indent!(indent), &v.name, v.data_type);
    }
    if !table.vars.is_empty() { println!(); }

    // Then, print all normal functions...
    for f in &table.funcs {
        println!("{}Function {}({}){} [", indent!(indent),
            &f.name,
            f.signature.args.iter().map(|a| format!("{}", a)).collect::<Vec<String>>().join(", "),
            if f.signature.ret != DataType::Void { format!(" -> {}", f.signature.ret) } else { String::new() },
        );

        // Write the nested table
        pass_table(&f.table, INDENT_SIZE + indent);
        println!("{}]", indent!(indent));
    }
    // ...and all tasks
    for t in &table.tasks {
        println!("{}Task<Compute> {}{}::{}({}){};", indent!(indent),
            t.package_name,
            if !t.package_version.is_latest() { format!("<{}>", t.package_version) } else { String::new() },
            &t.name,
            t.signature.args.iter().enumerate().map(|(i, a)| format!("{}: {}", t.arg_names[i], a)).collect::<Vec<String>>().join(", "),
            if t.signature.ret != DataType::Void { format!(" -> {}", t.signature.ret) } else { String::new() },
        );
    }
    if !table.vars.is_empty()|| !table.funcs.is_empty() || !table.tasks.is_empty() { println!(); }

    // Finally print the class definitions
    for c in &table.classes {
        println!("{}Class {}{} {{", indent!(indent), if let Some(package) = &c.package_name { format!("{}{}::", package, if !c.package_version.as_ref().unwrap().is_latest() { format!("<{}>", c.package_version.as_ref().unwrap()) } else { String::new() }) } else { String::new() }, &c.name);
        // Print all properties
        for p in &c.props {
            println!("{}property {}: {};", indent!(INDENT_SIZE + indent), &p.name, p.data_type);
        }
        // Print all functions
        for m in &c.methods {
            let f: &FunctionState = &table.funcs[*m];
            println!("{}method {}({}){};", indent!(INDENT_SIZE + indent),
                &f.name,
                f.signature.args.iter().map(|a| format!("{}", a)).collect::<Vec<String>>().join(", "),
                if f.signature.ret != DataType::Void { format!(" -> {}", f.signature.ret) } else { String::new() },
            );
        }
        println!("{}}};", indent!(indent));
    }
    if !table.vars.is_empty()|| !table.funcs.is_empty() || !table.tasks.is_empty() || !table.classes.is_empty() { println!(); }

    // _Finally_ finally, print the intermediate results
    for (name, avail) in &table.results {
        println!("{}IntermediateResult '{}' -> '{:?}'", indent!(INDENT_SIZE), name, avail);
    }

    // Done
}

/// Prints the extra function bodies of the Workflow.
/// 
/// # Arguments
/// - `workflow`: The UnresolvedWorkflow who's function buffers we print.
/// - `table`: The VirtualTableState that we use to resolve definitions.
/// - `indent`: The indent with which to print the function buffers.
/// 
/// # Returns
/// Nothing, but does print the function buffers to stdout.
pub fn pass_f_edges(workflow: &UnresolvedWorkflow, table: &mut VirtualTableState, indent: usize) {
    for (i, edges) in &workflow.f_edges {
        // Print the header, resolving it
        let f: &FunctionState = table.func(*i);
        println!("{}Function {}({}){} {{", indent!(indent),
            &f.name,
            f.signature.args.iter().map(|a| format!("{}", a)).collect::<Vec<String>>().join(", "),
            if f.signature.ret != DataType::Void { format!(" -> {}", f.signature.ret) } else { String::new() },
        );

        // Print the edge body (use the correct table!)
        table.push(&table.func(*i).table);
        pass_edges(edges, table, INDENT_SIZE + indent, HashSet::new());
        table.pop();

        // Print the closing brackets
        println!("{}}}", indent!(indent));
    }
}

/// Prints a given EdgeBuffer to stdout.
/// 
/// # Arguments
/// - `edges`: The EdgeBuffer to print.
/// - `table`: The VirtualTableState that we use to resolve indices.
/// - `indent`: The indent with which to print the buffer.
/// - `stop`: An optional set of nodes that we need to stop iterating for (useful for printing joining branches).
/// 
/// # Returns
/// Nothing, but does print the buffer to stdout.
pub fn pass_edges(edges: &EdgeBuffer, table: &mut VirtualTableState, indent: usize, stop: HashSet<EdgeBufferNodePtr>) {
    // We will write the edges in an instruction-like way, except that branches will be funky :#
    match edges.start() {
        Some(start) => {
            // Loop 'n' match
            let mut temp: Option<EdgeBufferNodePtr> = Some(start.clone());
            while temp.is_some() {
                // Get the next value
                let node: EdgeBufferNodePtr = temp.take().unwrap();

                // If we already had this one, stop
                if stop.contains(&node) { break; }
                // Print it
                let n: Ref<EdgeBufferNode> = node.borrow();
                pass_edge(&n.edge, table, indent);

                // Match on it
                match &n.next {
                    EdgeBufferNodeLink::Linear(next) => {
                        // Move to the next
                        temp = Some(next.clone());
                    },
                    EdgeBufferNodeLink::Branch(true_branch, false_branch, next) => {
                        // Add next to a copy of the hashset
                        let mut nested_stop: HashSet<EdgeBufferNodePtr> = stop.clone();
                        if let Some(next) = next { nested_stop.insert(next.clone()); }

                        // Print the header
                        print!("{}Branch {{", indent!(indent));
                        // Print the true branch
                        if let Some(true_branch) = true_branch {
                            println!();
                            pass_edges(&true_branch.into(), table, INDENT_SIZE + indent, nested_stop.clone());
                            print!("{}", indent!(indent));
                        }
                        print!("}} {{");
                        // Print the fales branch
                        if let Some(false_branch) = false_branch {
                            println!();
                            pass_edges(&false_branch.into(), table, INDENT_SIZE + indent, nested_stop.clone());
                            print!("{}", indent!(indent));
                        }
                        println!("}}");

                        // Continue with the next, if any
                        if let Some(next) = next { temp = Some(next.clone()); }
                    },
                    EdgeBufferNodeLink::Parallel(branches, join) => {
                        // Add join to a copy of the hashset
                        let mut nested_stop: HashSet<EdgeBufferNodePtr> = stop.clone();
                        nested_stop.insert(join.clone());

                        // Print the branches things
                        print!("{}Parallel", indent!(indent));
                        for b in branches {
                            println!(" {{");
                            pass_edges(&b.into(), table, INDENT_SIZE + indent, nested_stop.clone());
                            print!("{}}}", indent!(indent));
                        }
                        println!();

                        // Continue with the join, if any
                        temp = Some(join.clone());
                    },
                    EdgeBufferNodeLink::Loop(cond, body, next) => {
                        // Add next to a copy of the hashset
                        let mut nested_stop: HashSet<EdgeBufferNodePtr> = stop.clone();
                        if let Some(next) = next { nested_stop.insert(next.clone()); }

                        // Print the branches things
                        println!("{}Loop {{", indent!(indent));
                        pass_edges(&cond.into(), table, INDENT_SIZE + indent, nested_stop.clone());
                        println!("{}}} {{", indent!(indent));
                        if let Some(body) = body { pass_edges(&body.into(), table, INDENT_SIZE + indent, nested_stop); }
                        println!("{}}}", indent!(indent));

                        // Continue with the next, if any
                        if let Some(next) = next { temp = Some(next.clone()); }
                    },

                    // The rest doest not progress but print for niceness (if we feel like it)
                    EdgeBufferNodeLink::None => { println!("{}<None>", indent!(indent)); }
                    EdgeBufferNodeLink::End  => { println!("{}<End>", indent!(indent)); }
                    _                        => {},
                };
            }
        },
        None => { println!("{}<no edges>", indent!(indent)); }
    }
}

/// Prints a given Edge to stdout.
/// 
/// # Arguments
/// - `edge`: The Edge to print.
/// - `table`: The VirtualTableState that we use to resolve indices.
/// - `indent`: The indent with which to print the buffer.
/// 
/// # Returns
/// Nothing, but does print the edge to stdout.
pub fn pass_edge(edge: &Edge, table: &mut VirtualTableState, indent: usize) {
    // Match the Edge
    use Edge::*;
    match edge {
        Node{ task, .. } => {
            // Write the task as compute or transfer
            let task: &TaskState = table.task(*task);
            let task: String = format!("{}{}::{}", task.package_name, if !task.package_version.is_latest() { format!("<{}>", task.package_version) } else { String::new() }, task.name);

            // Write it
            println!("{}Node({})", indent!(indent), task);
        },
        Linear{ instrs, .. } => {
            // Print the instructions linearly
            print!("{}[", indent!(indent));
            let mut first: bool = true;
            for i in instrs {
                if first { first = false; }
                else { print!("\n{}", indent!(1 + indent)); }
                pass_edge_instr(i, table);
            }
            println!("]");
        },
        Stop{} => {
            println!("{}<Stop>", indent!(indent));
        },

        Call{ .. } => {
            println!("{}Call", indent!(indent));
        },
        Return{} => {
            println!("{}<Return>", indent!(indent));
        },

        // The rest doesn't have to be printed
        _ => {},
    }
}

/// Prints a given EdgeInstr to stdout.
/// 
/// # Arguments
/// - `instr`: The EdgeInstr to print.
/// - `table`: The VirtualTableState we use to resolve indices.
/// 
/// # Returns
/// Nothing, but does print the instruction to stdout.
pub fn pass_edge_instr(instr: &EdgeInstr, table: &mut VirtualTableState) {
    // Match the instruction
    use EdgeInstr::*;
    match instr {
        Cast{ res_type } => { print!("{} {}", instr, res_type); },

        Branch{ next }    => { print!("{} {}", instr, next); },
        BranchNot{ next } => { print!("{} {}", instr, next); },

        Proj{ field } => { print!("{} {}", instr, field); },

        Array{ length, res_type } => { print!("{} {},{}", instr, res_type, length); },
        ArrayIndex{ res_type }    => { print!("{} {}", instr, res_type); },
        Instance{ def }           => { print!("{} {}", instr, table.class(*def).name); },

        VarSet{ def } => { print!("{} {}", instr, table.var(*def).name); },
        VarGet{ def } => { print!("{} {}", instr, table.var(*def).name); },

        Boolean{ value } => { print!("{} {}", instr, value); },
        Integer{ value } => { print!("{} {}", instr, value); },
        Real{ value }    => { print!("{} {}", instr, value); },
        String{ value }  => { print!("{} \"{}\"", instr, value.replace('\n', "\\n").replace('\t', "\\t").replace('\r', "\\r").replace('\\', "\\\\").replace('\"', "\\\"")); },
        Function{ def }  => { print!("{} {}", instr, table.func(*def).name); },

        // Any other instruction is just printing it without any value
        instr => { print!("{}", instr); }
    }
}





/***** LIBRARY *****/
/// Starts printing the root of the AST (i.e., an UnresolvedWorkflow).
/// 
/// # Arguments
/// - `state`: The TableState that we use to resolve definition references.
/// - `root`: The root node of the tree on which this compiler pass will be done.
/// 
/// # Returns
/// The same root node as went in (since this compiler pass performs no transformations on the tree).
/// 
/// # Errors
/// This pass doesn't really error, but is here for convention purposes.
pub fn do_traversal(state: &CompileState, root: UnresolvedWorkflow) -> Result<UnresolvedWorkflow, Vec<Error>> {
    println!("UnresolvedWorkflow {{");

    // First up: print the workflow's table
    pass_table(&state.table, INDENT_SIZE);
    println!();
    println!();
    println!();

    // Print the function edges
    if !root.f_edges.is_empty() {
        pass_f_edges(&root, &mut VirtualTableState::with(&state.table), INDENT_SIZE);
        println!();
        println!();
        println!();
    }

    // Print the main function body
    pass_edges(&root.main_edges, &mut VirtualTableState::with(&state.table), INDENT_SIZE, HashSet::new());

    // Done
    println!("}}");
    Ok(root)
}

