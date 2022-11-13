//  SYMBOL TABLES.rs
//    by Lut99
// 
//  Created:
//    19 Aug 2022, 12:43:19
//  Last edited:
//    15 Sep 2022, 14:15:59
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements a traversal that prints all symbol tables neatly for a
//!   given program.
// 

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use brane_dsl::SymbolTable;
use brane_dsl::symbol_table::{ClassEntry, FunctionEntry, VarEntry};
use brane_dsl::ast::{Block, Program, Stmt};

pub use crate::errors::AstError as Error;


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
/// Prints a Stmt node.
/// 
/// # Arguments
/// - `stmt`: The Stmt to traverse.
/// - `indent`: The current base indent of all new lines to write.
/// 
/// # Returns
/// Nothing, but does print it.
fn pass_stmt(stmt: &Stmt, indent: usize) {
    // Match on the statement itself
    use Stmt::*;
    match stmt {
        Block{ block } => {
            // Simply print this one's symbol table
            print!("{}__nested_block: ", indent!(indent));
            pass_block(block, indent);
            println!();
        },

        FuncDef{ ident, code, .. } => {
            // Print the code block's symbol table
            print!("{}Function '{}': ", indent!(indent), ident.value);
            pass_block(code, indent);
            println!();
        },
        ClassDef{ methods, .. } => {
            // Recurse into the methods
            for m in methods.iter() {
                pass_stmt(m, indent);
            }
        },
    
        If{ consequent, alternative, .. } => {
            // Print the symbol tables of the consequent and (optionally) the alternative
            print!("{}If ", indent!(indent));
            pass_block(consequent, indent);
            if let Some(alternative) = alternative {
                print!(" Else ");
                pass_block(alternative, indent);
            }
            println!();
        },
        For{ consequent, .. } => {
            // Print the symbol table of the consequent
            print!("{}For ", indent!(indent));
            pass_block(consequent, indent);
            println!();
        },
        While{ consequent, .. } => {
            // Print the block
            print!("{}While ", indent!(indent));
            pass_block(consequent, indent);
            println!();
        },
        On{ block, .. } => {
            // Print the block
            print!("{}On ", indent!(indent));
            pass_block(block, indent);
            println!();
        },
        Parallel{ blocks, .. } => {
            // Print the blocks
            println!("{}Parallel [", indent!(indent));
            for b in blocks {
                pass_stmt(b, indent + 3);
            }
            println!("{}]", indent!(indent));
        },

        // We don't care about the rest
        _ => {}
    }
}

/// Prints a Block node.
/// 
/// # Arguments
/// - `block`: The Block to traverse.
/// - `indent`: The current base indent of all new lines to write.
/// 
/// # Returns
/// Nothing, but does print it.
fn pass_block(block: &Block, indent: usize) {
    // Print the current symbol table
    println!("[");
    pass_symbol_table(&block.table, indent + INDENT_SIZE);

    // Now we print the following symbol tables with additional indentation
    let st: Ref<SymbolTable> = block.table.borrow();
    if !block.stmts.is_empty() && (st.has_functions() || st.has_classes() || st.has_variables()) { println!(); }
    for stmt in block.stmts.iter() {
        pass_stmt(stmt, indent + INDENT_SIZE);
    }

    // Done
    print!("{}]", indent!(indent));
}

/// Prints a SymbolTable.
/// 
/// # Arguments
/// - `symbol_table`: The SymbolTable to traverse.
/// - `indent`: The current base indent of all new lines to write.
/// 
/// # Returns
/// Nothing, but does print it.
fn pass_symbol_table(symbol_table: &Rc<RefCell<SymbolTable>>, indent: usize) {
    // Borrow the table
    let st: Ref<SymbolTable> = symbol_table.borrow();

    // First, print all of its functions
    for (name, f) in st.functions() {
        let f: Ref<FunctionEntry> = f.borrow();
        println!("{}{}func {}{}{}",
            indent!(indent),
            if f.index != usize::MAX { format!("{}) ", f.index) } else { String::new() },
            if let Some(pkg) = &f.package_name { format!("{}::", pkg) } else { String::new() },
            name,
            f.signature
        );
    }
    // Next, print all of its classes
    for (_, c) in st.classes() {
        let c: Ref<ClassEntry> = c.borrow();

        // Print the class signature header
        println!("{}{}class {}{} {{",
            indent!(indent),
            if c.index != usize::MAX { format!("{}) ", c.index) } else { String::new() },
            if let Some(pkg) = &c.package_name { format!("{}::", pkg) } else { String::new() },
            c.signature
        );
        // Print the associated symbol table
        pass_symbol_table(&c.symbol_table, indent + INDENT_SIZE);
        // Print the closing thing done
        println!("{}}}", indent!(indent));
    }
    // Finally, print the variables
    for (name, v) in st.variables() {
        let v: Ref<VarEntry> = v.borrow();
        println!("{}{}var {} : {},", indent!(indent), if v.index != usize::MAX { format!("{}) ", v.index) } else { String::new() }, name, v.data_type);
    }

    // Done
}





/***** LIBRARY *****/
/// Starts printing the root of the AST (i.e., a series of statements).
/// 
/// # Arguments
/// - `root`: The root node of the tree on which this compiler pass will be done.
/// 
/// # Returns
/// The same root node as went in (since this compiler pass performs no transformations on the tree).
/// 
/// # Errors
/// This pass generally doesn't error, but is here for convention purposes.
pub fn do_traversal(root: Program) -> Result<Program, Vec<Error>> {
    // Iterate over all statements and run the appropriate match
    print!("__root ");
    pass_block(&root.block, 0);
    println!();

    // Done
    Ok(root)
}
