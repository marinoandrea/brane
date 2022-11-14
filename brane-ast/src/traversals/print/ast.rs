//  AST.rs
//    by Lut99
// 
//  Created:
//    31 Aug 2022, 09:25:11
//  Last edited:
//    14 Nov 2022, 10:23:06
//  Auto updated?
//    Yes
// 
//  Description:
//!   Prints the `brane-ast` AST.
// 

use std::collections::HashSet;

pub use crate::errors::AstError as Error;
use crate::data_type::DataType;
use crate::ast::{Edge, EdgeInstr, FunctionDef, SymTable, TaskDef, Workflow};
use crate::state::VirtualSymTable;


/***** MACROS ******/
/// Generates the correct number of spaces for an indent.
macro_rules! indent {
    ($n_spaces:expr) => {
        ((0..$n_spaces).map(|_| ' ').collect::<String>())
    };
}

/// Generates a properly padded line number.
macro_rules! line_number {
    ($l:expr) => {
        format!("{:>1$}", $l, LINE_SIZE)
    };
}





/***** CONSTANTS *****/
/// Determines the number of decimals the line numbers will always have
const LINE_SIZE: usize = 4;
/// Determines the increase in indentation for every nested level.
const INDENT_SIZE: usize = 4;





/***** TRAVERSAL FUNCTIONS *****/
/// Prints the global table of the Workflow.
/// 
/// # Arguments
/// - `table`: The SymTable we print.
/// - `indent`: The indent with which to print the table.
/// 
/// # Returns
/// Nothing, but does print the table to stdout.
fn pass_table(table: &SymTable, indent: usize) {
    // Simply print all fields in a less-cluttering-to-most-cluttering order

    // Variables first
    for v in &table.vars {
        println!("{}Var {}: {};", indent!(indent), &v.name, v.data_type);
    }
    if !table.vars.is_empty() { println!(); }

    // Then, print all normal functions...
    for f in &table.funcs {
        print!("{}Function {}({}){}", indent!(indent),
            &f.name,
            f.args.iter().map(|a| format!("{}", a)).collect::<Vec<String>>().join(", "),
            if f.ret != DataType::Void { format!(" -> {}", f.ret) } else { String::new() },
        );

        // If the function has a (meaningful) nested state, print that too
        if !f.table.funcs.is_empty() || !f.table.tasks.is_empty() || !f.table.classes.is_empty() || !f.table.vars.is_empty() {
            println!(" [");
            pass_table(&f.table, INDENT_SIZE + indent);
            print!("{}]", indent!(indent));
        }
        println!(";");
    }
    // ...and all tasks
    for t in &table.tasks {
        match t {
            TaskDef::Compute { package, version, function, args_names }  => {
                println!("{}Task<Compute> {}{}::{}({}){};", indent!(indent),
                    package,
                    if !version.is_latest() { format!("<{}>", version) } else { String::new() },
                    &function.name,
                    function.args.iter().enumerate().map(|(i, a)| format!("{}: {}", args_names[i], a)).collect::<Vec<String>>().join(", "),
                    if function.ret != DataType::Void { format!(" -> {}", function.ret) } else { String::new() },
                );
            },
            TaskDef::Transfer {} => println!("Task<Transfer>;"),
        }
    }
    if !table.vars.is_empty() || !table.funcs.is_empty() || !table.tasks.is_empty() { println!(); }

    // Finally print the class definitions
    for c in &table.classes {
        println!("{}Class {}{} {{", indent!(indent), if let Some(package) = &c.package { format!("{}::", package) } else { String::new() }, &c.name);
        // Print all properties
        for p in &c.props {
            println!("{}property {}: {};", indent!(INDENT_SIZE + indent), &p.name, p.data_type);
        }
        // Print all functions
        for m in &c.methods {
            let m: &FunctionDef = &table.funcs[*m];
            println!("{}method &{}({}){};", indent!(INDENT_SIZE + indent),
                &m.name,
                m.args.iter().map(|a| format!("{}", a)).collect::<Vec<String>>().join(", "),
                if m.ret != DataType::Void { format!(" -> {}", m.ret) } else { String::new() },
            );
        }
        println!("{}}};", indent!(indent));
    }
    if !table.vars.is_empty() || !table.funcs.is_empty() || !table.tasks.is_empty() || !table.classes.is_empty() { println!(); }

    // And the results
    for (name, avail) in table.results.iter() {
        println!("{}IntermediateResult '{}' -> '{:?}'", indent!(INDENT_SIZE), name, avail);
    }

    // Done
}

/// Prints a given Edge buffer to stdout.
/// 
/// # Arguments
/// - `index`: The starting index in the edges list to follow.
/// - `edges`: The list of Edges to print.
/// - `table`: The VirtualSymTable we use to resolve indices.
/// - `indent`: The indent with which to print the buffer.
/// - `done`: A set of nodes we've already seen that we use to avoid getting stuck forever.
/// 
/// # Returns
/// Nothing, but does print the buffer to stdout.
fn pass_edges(index: usize, edges: &[Edge], table: &VirtualSymTable, indent: usize, done: &mut HashSet<usize>) {
    // Loop until we are out-of-bounds or encounter a stop
    let mut i: usize = index;
    while i < edges.len() {
        // If we've alread done this one, stop
        if done.contains(&i) {
            println!("{} {}<Link to {}>", indent!(LINE_SIZE), indent!(indent), i);
            break;
        }
        done.insert(i);

        // Get the node's value
        let node: &Edge = &edges[i];

        // Match on it
        use Edge::*;
        match node {
            Node { task, locs, at, input, result, next } => {
                // Write the Node as a task call
                println!("{} {}Node({}){}{}{}",
                    line_number!(i),
                    indent!(indent),
                    match &table.task(*task) {
                        TaskDef::Compute { package, version, function, .. } => format!("{}{}::{}", package, if !version.is_latest() { format!("<{}>", version) } else { String::new() }, function.name),
                        TaskDef::Transfer {}                                => "__builtin::transfer".into(),
                    },
                    if locs.is_restrictive() { format!(" <limited to: {}>", locs.restricted().join(",")) } else { String::new() },
                    if let Some(at) = at { format!(" @{}", at) } else { String::new() },
                    if !input.is_empty() || result.is_some() { format!(" [{} -> {}]",
                        if !input.is_empty() { input.iter().map(|(name, avail)| format!("'{}'{}", name, if let Some(avail) = avail { format!(" ({:?})", avail) } else { String::new() })).collect::<Vec<String>>().join(", ").to_string() } else { "''".into() },
                        if let Some(name) = result { format!("'{}'", name) } else { "''".into() },
                    ) } else { String::new() },
                );

                // Move to the next node
                i = *next;
            },
            Linear { instrs, next } => {
                // Write the instructions within this edge
                print!("{} {}[", line_number!(i), indent!(indent));
                let mut first: bool = true;
                for i in instrs {
                    if first { first = false; }
                    else { print!("\n{} {} ", indent!(LINE_SIZE), indent!(indent)); }
                    pass_edge_instr(i, table);
                }
                println!("]");

                // Move to the next node
                i = *next;
            },
            Stop {} => {
                // Write a simple stop
                println!("{} {}<Stop>", line_number!(i), indent!(indent));

                // Do still move to the next node, if any
                i += 1;
            },

            Branch { true_next, false_next, merge } => {
                // Add the merge point to the 'already done' map for this loop
                let rem_merge: Option<bool> = merge.as_ref().map(|m| done.insert(*m));

                // Write the two branches of the branch
                print!("{} {}Branch {{", line_number!(i), indent!(indent));
                if merge.is_none() || merge.as_ref().unwrap() != true_next {
                    println!();
                    pass_edges(*true_next, edges, table, INDENT_SIZE + indent, done);
                    print!("{} {}", indent!(LINE_SIZE), indent!(indent));
                }
                print!("}} {{");
                if false_next.is_some() && (merge.is_none() || merge.as_ref().unwrap() != false_next.as_ref().unwrap()) {
                    println!();
                    pass_edges(false_next.unwrap(), edges, table, INDENT_SIZE + indent, done);
                    print!("{} {}", indent!(LINE_SIZE), indent!(indent));
                }
                println!("}}");

                // Remove the 'already done' to make sure it is written next
                if rem_merge.is_some() && rem_merge.unwrap() {
                    done.remove(merge.as_ref().unwrap());
                }

                // Move to the next node
                match merge {
                    Some(merge) => { i = *merge; },
                    None        => { i += 1 },
                }
            },
            Parallel { branches, merge } => {
                // Add the merge point to the 'already done' map for this loop
                let rem_merge: bool = done.insert(*merge);

                // Write the branches of the branch, in sequence
                print!("{} {}Parallel {{", line_number!(i), indent!(indent));
                let mut first: bool = true;
                for i in 0..branches.len() {
                    if *merge != branches[i] {
                        // Add the next branch
                        let rem_merge: bool = if i < branches.len() - 1 { done.insert(branches[i + 1]) } else { false };

                        // Print it
                        if first { first = false; }
                        else { print!("}} {{"); }
                        println!();
                        pass_edges(branches[i], edges, table, INDENT_SIZE + indent, done);
                        print!("{} {}", indent!(LINE_SIZE), indent!(indent));

                        // Remove the next one again
                        if rem_merge { done.remove(&branches[i + 1]); }
                    }
                }
                println!("}}");

                // Remove the 'already done' to make sure it is written next
                if rem_merge { done.remove(merge); }

                // Move to the next node
                i = *merge;
            },
            Join { merge, next } => {
                // Write the join itself
                println!("{} {}Join({:?})", line_number!(i), indent!(indent), *merge);

                // Move to the next node
                i = *next;
            },

            Loop { cond, body, next } => {
                // Write the loop
                println!("{} {}Loop {{", line_number!(i), indent!(indent));
                pass_edges(*cond, edges, table, INDENT_SIZE + indent, done);
                print!("{} {}}} {{", indent!(LINE_SIZE), indent!(indent));
                if next.is_none() || body != next.as_ref().unwrap() {
                    println!();
                    pass_edges(*body, edges, table, INDENT_SIZE + indent, done);
                    print!("{} {}", indent!(LINE_SIZE), indent!(indent));
                }
                println!("}}");

                // Move to the next node
                match next {
                    Some(next) => { i = *next; },
                    None       => { i += 1; },
                }
            },

            Call { next } => {
                // Just print it
                println!("{} {}Call", line_number!(i), indent!(indent));

                // Move to the next node
                i = *next;
            },
            Return{} => {
                println!("{} {}Return", line_number!(i), indent!(indent));
                i += 1
            },
        }
    }
}

/// Prints a given EdgeInstr to stdout.
/// 
/// # Arguments
/// - `instr`: The EdgeInstr to print.
/// - `table`: The VirtualSymTable we use to resolve indices.
/// 
/// # Returns
/// Nothing, but does print the instruction to stdout.
fn pass_edge_instr(instr: &EdgeInstr, table: &VirtualSymTable) {
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

        VarGet{ def } => { print!("{} {}", instr, table.var(*def).name); },
        VarSet{ def } => { print!("{} {}", instr, table.var(*def).name); },

        Boolean{ value } => { print!("{} {}", instr, value); },
        Integer{ value } => { print!("{} {}", instr, value); },
        Real{ value }    => { print!("{} {}", instr, value); },
        String{ value }  => { print!("{} \"{}\"", instr, value.replace('\\', "\\\\").replace('\n', "\\n").replace('\t', "\\t").replace('\r', "\\r").replace('\"', "\\\"")); },
        Function{ def }  => { print!("{} {}", instr, table.func(*def).name); },

        // Any other instruction is just printing it without any value
        instr => { print!("{}", instr); }
    }
}





/***** LIBRARY *****/
/// Starts printing the root of the AST (i.e., a Workflow).
/// 
/// # Arguments
/// - `root`: The root node of the tree on which this compiler pass will be done.
/// 
/// # Returns
/// The same root node as went in (since this compiler pass performs no transformations on the tree).
/// 
/// # Errors
/// This pass doesn't really error, but is here for convention purposes.
pub fn do_traversal(root: Workflow) -> Result<Workflow, Vec<Error>> {
    println!("Workflow {{");

    // First up: print the workflow table
    pass_table(&root.table, INDENT_SIZE);
    if !root.table.vars.is_empty() || !root.table.funcs.is_empty() || !root.table.tasks.is_empty() || !root.table.classes.is_empty() || !root.table.results.is_empty() {
        println!();
        println!();
        println!();
    }

    // Print the main function body (and thus all function bodies)
    println!("{}<Main>", indent!(INDENT_SIZE));
    let mut table: VirtualSymTable = VirtualSymTable::with(&root.table);
    pass_edges(0, &root.graph, &table, INDENT_SIZE, &mut HashSet::new());

    // Print the functions
    for (i, f) in root.funcs.iter() {
        println!();
        println!("{}<Function {} ({})>", indent!(INDENT_SIZE), *i, root.table.funcs[*i].name);
        table.push(&table.func(*i).table);
        pass_edges(0, f, &table, INDENT_SIZE, &mut HashSet::new());
        table.pop();
    }

    // Done
    println!("}}");
    Ok(root)
}
