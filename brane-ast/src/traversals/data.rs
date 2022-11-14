//  DATA.rs
//    by Lut99
// 
//  Created:
//    25 Oct 2022, 13:34:31
//  Last edited:
//    14 Nov 2022, 09:27:01
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements a traversal that analyses data dependencies for external
//!   calls.
// 

use std::cell::Ref;
use std::collections::HashSet;

use uuid::Uuid;

use brane_dsl::DataType;
use brane_dsl::symbol_table::{ClassEntry, FunctionEntry, SymbolTableEntry};
use brane_dsl::ast::{Block, Data, Expr, Program, Stmt};

use crate::errors::AstError;
use crate::spec::BuiltinClasses;
use crate::state::{CompileState, DataState};


/***** TESTS *****/
#[cfg(test)]
mod tests {
    use brane_dsl::ParserOptions;
    use brane_shr::utilities::{create_data_index, create_package_index, test_on_dsl_files};
    use specifications::data::DataIndex;
    use specifications::package::PackageIndex;
    use super::*;
    use super::super::print::symbol_tables;
    use crate::{compile_snippet_to, CompileResult, CompileStage};
    use crate::state::CompileState;


    /// Tests the traversal by generating symbol tables for every file.
    #[test]
    fn test_data() {
        test_on_dsl_files("BraneScript", |path, code| {
            // Start by the name to always know which file this is
            println!("{}", (0..80).map(|_| '-').collect::<String>());
            println!("File '{}' gave us:", path.display());

            // Load the package index
            let pindex: PackageIndex = create_package_index();
            let dindex: DataIndex    = create_data_index();

            // Run up to this traversal
            let mut state: CompileState = CompileState::new();
            let program: Program = match compile_snippet_to(&mut state, code.as_bytes(), &pindex, &dindex, &ParserOptions::bscript(), CompileStage::Data) {
                CompileResult::Program(p, warns) => {
                    // Print warnings if any
                    for w in warns {
                        w.prettyprint(path.to_string_lossy(), &code);
                    }
                    p
                },
                CompileResult::Eof(err) => {
                    // Print the error
                    err.prettyprint(path.to_string_lossy(), &code);
                    panic!("Failed to flatten symbol tables (see output above)");
                }
                CompileResult::Err(errs) => {
                    // Print the errors
                    for e in errs {
                        e.prettyprint(path.to_string_lossy(), &code);
                    }
                    panic!("Failed to flatten symbol tables (see output above)");
                },

                _ => { unreachable!(); },
            };

            // Now print the file for prettyness
            symbol_tables::do_traversal(program).unwrap();
            // println!("{}\n", (0..40).map(|_| "- ").collect::<String>());
            // print_state(&state.table, 0);
            println!("{}\n\n", (0..80).map(|_| '-').collect::<String>());
        });
    }
}





/***** TRAVERSAL FUNCTIONS *****/
/// Attempts to analyse the data dependencies for this block.
/// 
/// # Arguments
/// - `block`: The Block to traverse.
/// - `table`: The DataTable we use to keep track of which variable has what value.
/// 
/// # Returns
/// This functions returns the possible datasets that are _returned_ in this block. This is thus different from `pass_expr()`.
fn pass_block(block: &mut Block, table: &mut DataState) -> HashSet<Data> {
    // Iterate over all the statements
    let mut ids: HashSet<Data> = HashSet::new();
    for s in &mut block.stmts {
        let sids: HashSet<Data> = pass_stmt(s, table);
        ids.extend(sids);
    }

    // Done
    ids
}

/// Attempts to analyse the data dependencies for this statement.
/// 
/// # Arguments
/// - `stmt`: The Stmt to traverse.
/// - `table`: The DataTable we use to keep track of which variable has what value.
/// 
/// # Returns
/// This functions returns the possible datasets that are _returned_ in this statement. This is thus different from `pass_expr()`.
fn pass_stmt(stmt: &mut Stmt, table: &mut DataState) -> HashSet<Data> {
    // Match on the exact statement
    use Stmt::*;
    match stmt {
        Block{ block, .. } => {
            pass_block(block, table)
        },

        FuncDef{ code, st_entry, .. } => {
            let ids: HashSet<Data> = pass_block(code, table);

            // Push the results to the data table
            table.set_funcs(&st_entry.as_ref().unwrap().borrow().name, ids);

            // The definition itself doesn't return, so it doesn't introduce new identifiers
            HashSet::new()
        },
        ClassDef{ methods, .. } => {
            // Simply recurse, that'll do it (we are not interested in the results, since this function never returns anyway)
            for m in methods {
                pass_stmt(m, table);
            }

            // The definition itself doesn't return, so it doesn't introduce new identifiers
            HashSet::new()
        },
        Return{ expr, .. } => {
            if let Some(expr) = expr {
                // Return whether the expression returns any datasets
                pass_expr(expr, table)
            } else {
                // Otherwise, it doesn't return any new identifiers
                HashSet::new()
            }
        },

        If{ cond, consequent, alternative, .. } => {
            // We don't care about the condition, but recurse it for any inter-expression dependencies
            pass_expr(cond, table);

            // Next, we do different things depending on whether there is an alternative
            if let Some(alternative) = alternative {
                // If it's both, we first split the table in two halves, since any values added in the true block are not possible to obtain in the false block
                let mut false_table = table.clone();

                // Next, run the blocks with their own tables
                let mut ids: HashSet<Data> = pass_block(consequent, table);
                ids.extend(pass_block(alternative, &mut false_table));

                // Merge the table together again to get the post-if possibilities
                table.extend(false_table);

                // Now return the list of possible returns
                ids
            } else {
                // If it's only the consequent, we just do that block
                pass_block(consequent, table)
            }
        },
        For{ initializer, condition, increment, consequent, .. } => {
            // Do the initializer, condition and increment for traversal purposes (the order makes sense, I think - if we ever get weird behaviour, check here)
            pass_stmt(initializer, table);
            pass_expr(condition, table);
            pass_stmt(increment, table);

            // The last one has the returns we're interested in
            pass_block(consequent, table)
        },
        While{ condition, consequent, .. } => {
            // The condition is recursed only to resolve in-condition dependencies
            pass_expr(condition, table);

            // The block is what we care about
            pass_block(consequent, table)
        },
        On{ block, .. } => {
            // The location is a literal, so we skip

            // Do the block
            pass_block(block, table)
        },
        Parallel{ blocks, st_entry, .. } => {
            // The parallel _does_ return, Tim - or at least, we have to put it in the variable if there is one
            let mut ids: HashSet<Data> = HashSet::new();
            for b in blocks {
                ids.extend(pass_stmt(b, table));
            }

            // Put it in the variable if this Parallel is returning
            if let Some(st_entry) = st_entry {
                table.set_vars(&st_entry.borrow().name, ids);
            }

            // It never returns (since any returns it has are parallel-local)
            HashSet::new()
        },

        LetAssign{ value, st_entry, .. } |
        Assign{ value, st_entry, .. }    => {
            // Traverse the value
            let ids: HashSet<Data> = pass_expr(value, table);

            // Propagate that to the variable's data table (overwriting any previous, since this is an overwrite as well. If updated? -> that's why we recurse with the old table)
            table.set_vars(&st_entry.as_ref().unwrap().borrow().name, ids);

            // The statement itself never returns, though
            HashSet::new()
        },
        Expr{ expr, .. } => {
            // Recurse but never return
            pass_expr(expr, table);
            HashSet::new()
        },

        // The rest no matter
        _ => HashSet::new(),
    }
}

/// Attempts to analyse the data dependencies for this expression.
/// 
/// # Arguments
/// - `expr`: The Expr to traverse.
/// - `table`: The DataTable we use to keep track of which variable has what value.
/// 
/// # Returns
/// This function returns the possible identifiers that the evaluation of this expression can be if it concerns a Data or IntermediateResult. Note that this differs from `pass_block()` and `pass_stmt()`.
fn pass_expr(expr: &mut Expr, table: &DataState) -> HashSet<Data> {
    use Expr::*;
    match expr {
        Cast{ expr, .. } => {
            // Only dataset casts are allowed if it is a dataset itself; so we can simply recurse it
            pass_expr(expr, table)
        },

        Call{ args, input, result, st_entry, .. } => {
            // Populating calls is what this traversal is all about, so let's dive into the interesting stuff

            // Find out if this call is external
            let is_external: bool = if let Some(st_entry) = st_entry {
                let entry: Ref<FunctionEntry> = st_entry.borrow();
                entry.package_name.is_some()
            } else {
                false
            };

            // Only do interesting stuff if this function _is_ external, though
            if is_external {
                // Traverse into the arguments to find the input identifiers
                let mut ids: HashSet<Data> = HashSet::new();
                for a in args {
                    ids.extend(pass_expr(a, table))
                }
                *input = ids.into_iter().collect();

                // If this function returns an IntermediateResult, generate the ID while at it
                let entry: Ref<FunctionEntry> = st_entry.as_ref().unwrap().borrow();
                if entry.signature.ret == DataType::Class(BuiltinClasses::IntermediateResult.name().into()) {
                    // If this call is an external one _and_ it returns a result, we want to note it as such.

                    // Generate the identifier for this result
                    let uuid : String = Uuid::new_v4().to_string()[..6].into();
                    let id   : String = format!("result_{}_{}", entry.name, uuid);

                    // Note it in the function
                    *result = Some(id.clone());

                    // Return the identifier to return from this call
                    HashSet::from([ Data::IntermediateResult(id) ])
                } else {
                    HashSet::new()
                }

            } else {
                // Still recurse into the arguments to catch any nested calls
                for a in args {
                    pass_expr(a, table);
                }

                // The returned identifier is quite simply that of the function itself
                if let Some(st_entry) = st_entry {
                    table.get_func(&st_entry.borrow().name).clone()
                } else {
                    HashSet::new()
                }

            }
        },
        Array{ values, .. } => {
            // We are lazy, and accept state space explosion in case someone is so nuts to have an array of Data
            let mut ids: HashSet<Data> = HashSet::new();
            for v in values {
                ids.extend(pass_expr(v, table));
            }
            ids
        },
        ArrayIndex{ array, index, .. } => {
            // Do the array first, and remember that to return
            let ids: HashSet<Data> = pass_expr(array, table);
            // We do the other side for fun as well
            pass_expr(index, table);

            // But return the ids of the array expression, that's importat
            ids
        },

        UnaOp{ expr, .. } => {
            // Simply recurse, since there aren't really any expressions possible on datasets and such
            pass_expr(expr, table)
        },
        BinOp{ lhs, rhs, .. } => {
            // There's not really a data-changing operation, so just join and we assume it won't really matter
            let mut ids: HashSet<Data> = pass_expr(lhs, table);
            ids.extend(pass_expr(rhs, table));
            ids
        },
        Proj{ st_entry, .. } => {
            // The projection is a stand-in for a variable, so we'd like the current value of that one
            if let Some(st_entry) = st_entry {
                match st_entry {
                    SymbolTableEntry::FunctionEntry(_) |
                    SymbolTableEntry::ClassEntry(_)    => {
                        // Although the entries are interesting, the projection itself doesn't return a value, so no data chances
                        HashSet::new()
                    },

                    SymbolTableEntry::VarEntry(st_entry) => {
                        // Return the matching value for the referenced variable here
                        table.get_var(&st_entry.borrow().name).clone()
                    }
                }
            } else {
                HashSet::new()
            }
        },

        Instance{ properties, st_entry, .. } => {
            // Note down whether this happens to be a Data or an IntermediateResult
            let is_data: bool = {
                let entry: Ref<ClassEntry> = st_entry.as_ref().unwrap().borrow();
                if &entry.signature.name == BuiltinClasses::IntermediateResult.name() { panic!("Didn't expect an explicit IntermediateResult instantiation"); }
                &entry.signature.name == BuiltinClasses::Data.name()
            };

            // Recurse into the properties to traverse the expressions there
            let mut name: Option<String> = None;
            for p in properties {
                pass_expr(&mut p.value, table);

                // While at it, note if we find 'name' - and if we do, its value
                if is_data && &p.name.value == "name" {
                    name = Some(if let Expr::Literal{ literal: brane_dsl::ast::Literal::String{ value, .. } } = &*p.value {
                        value.clone()
                    } else {
                        panic!("Expected a String literal as Data/IntermediateResult `name` property, got {:?}", &*p.value);
                    })
                }
            }

            // If we are a data, then return the name as an identifier
            if is_data {
                if let Some(id) = name {
                    HashSet::from([ Data::Data(id) ])
                } else {
                    panic!("Got a Data/IntermediateResult without a `name`; this should never happen");
                }
            } else {
                HashSet::new()
            }
        },
        VarRef{ st_entry, .. } => {
            // In this case, simply return the value in the table
            table.get_var(&st_entry.as_ref().unwrap().borrow().name).clone()
        },

        // Any others are never returning anything of interest
        _ => HashSet::new(),
    }
}





/***** LIBRARY *****/
/// Analyses data dependencies in the given `brane-dsl` AST.
/// 
/// Note that type analysis must already have been performed.
/// 
/// # Arguments
/// - `root`: The root node of the tree on which this compiler pass will be done.
/// 
/// # Returns
/// The same nodes as went in, but now with added in `input` and `result` annotations to each external call.
/// 
/// # Errors
/// This pass typically does not error, but the option is here for convention purposes.
pub fn do_traversal(state: &mut CompileState, root: Program) -> Result<Program, Vec<AstError>> {
    let mut root = root;

    // Iterate over all statements to analyse dependencies
    for s in root.block.stmts.iter_mut() {
        pass_stmt(s, &mut state.data);
    }

    // Done
    Ok(root)
}

