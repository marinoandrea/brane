//  DSL.rs
//    by Lut99
// 
//  Created:
//    18 Aug 2022, 13:46:22
//  Last edited:
//    21 Sep 2022, 14:19:56
//  Auto updated?
//    Yes
// 
//  Description:
//!   Prints the `brane-dsl` AST in BraneScript-like Syntax.
// 

use brane_dsl::location::AllowedLocations;
use brane_dsl::ast::{self as dsl_ast, Block, Expr, Identifier, Literal, Program, Property, PropertyExpr, Stmt};

pub use crate::errors::AstError as Error;


/***** TESTS *****/
#[cfg(test)]
pub mod tests {
    use brane_dsl::ParserOptions;
    use brane_shr::utilities::{create_data_index, create_package_index, test_on_dsl_files};
    use specifications::data::DataIndex;
    use specifications::package::PackageIndex;
    use super::*;
    use crate::{compile_program_to, CompileResult, CompileStage};


    /// 'Tests' the traversal by printing the AST for every node.
    #[test]
    fn test_print() {
        test_on_dsl_files("BraneScript", |path, code| {
            println!("{}", (0..80).map(|_| '-').collect::<String>());
            println!("File '{}' gave us:", path.display());

            // Load the package index
            let pindex: PackageIndex = create_package_index();
            let dindex: DataIndex    = create_data_index();

            // Run up to this traversal
            let program: Program = match compile_program_to(code.as_bytes(), &pindex, &dindex, &ParserOptions::bscript(), CompileStage::None) {
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
                    panic!("Failed to parse file (see output above)");
                }
                CompileResult::Err(errs) => {
                    // Print the errors
                    for e in errs {
                        e.prettyprint(path.to_string_lossy(), &code);
                    }
                    panic!("Failed to parse file (see output above)");
                },

                _ => { unreachable!(); },
            };

            // Now print the tree
            do_traversal(program).unwrap();
            println!("{}\n\n", (0..80).map(|_| '-').collect::<String>());
        });
    }
}





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
pub fn pass_stmt(stmt: &Stmt, indent: usize) {
    // Match on the statement itself
    use Stmt::*;
    match stmt {
        Block{ block } => {
            // Pass over the block instead, but do print the indentation first.
            print!("{}", indent!(indent));
            pass_block(&*block, indent);
            println!();
        },

        Import { name, version, .. } => {
            // Print as an import statement
            print!("{}import ", indent!(indent));
            // Print the identifier
            pass_identifier(name);
            // Print the version, optionally
            if let Literal::Semver{ range, .. } = &version {
                if range.is_some() {
                    print!("[");
                    pass_literal(version);
                    print!("]");
                }
            } else {
                panic!("Got a non-Semver Literal '{:?}' in an import statement; this should never happen!", version);
            }
            // Do newline
            println!();
        },
        FuncDef{ ident, params, code, .. } => {
            // Print the 'func' prefix
            print!("{}func ", indent!(indent));
            // Print the identifier
            pass_identifier(ident);
            // Print the parameters
            print!("(");
            let mut first = true;
            for p in params {
                if first { first = false; }
                else { print!(", "); }
                pass_identifier(p);
            }
            // Print the block
            print!(") ");
            pass_block(&code, indent);
            println!();
        },
        ClassDef{ ident, props, methods, .. } => {
            // Print the 'class' prefix
            print!("{}class ", indent!(indent));
            // Print the identifier
            pass_identifier(ident);
            // Print the class opening
            println!(" {{");
            // Print the properties
            let largest_prop: usize = props.iter().map(|p| p.name.value.len()).max().unwrap_or(0);
            for p in props {
                pass_property(p, largest_prop, indent + 3);
            }
            // Print a newline if any properties have been written
            if !props.is_empty() { println!(); }
            // Print the methods
            for m in methods {
                pass_stmt(m, indent + 3);
            }
            // Finally, print the closing bracket
            println!("{}}}", indent!(indent));
        },
        Return{ expr, .. } => {
            // Print the return
            print!("{}return", indent!(indent));
            // If there is an expression, print it
            if let Some(expr) = expr {
                print!(" ");
                pass_expr(expr, indent);
            }
            // Print the semicolon
            println!(";");
        },
    
        If{ cond, consequent, alternative, .. } => {
            // Print the if first + its condition
            print!("{}if (", indent!(indent));
            pass_expr(cond, indent);
            print!(") ");
            // Print the if-block
            pass_block(&*consequent, indent);
            // If there is an else, do that
            if let Some(alternative) = alternative {
                print!(" else ");
                pass_block(&*alternative, indent);
            }
            println!();
        },
        For{ initializer, condition, increment, consequent, .. } => {
            // Print the three for parts
            print!("{}for (", indent!(indent));
            pass_stmt(initializer, indent);
            print!(" ");
            pass_expr(condition, indent);
            print!("; ");
            pass_stmt(increment, indent);
            print!(") ");
            // Print the block
            pass_block(&*consequent, indent);
            println!();
        },
        While{ condition, consequent, .. } => {
            // Print the while + its condition
            print!("{}while (", indent!(indent));
            pass_expr(condition, indent);
            print!(") ");
            // Print the block
            pass_block(&*consequent, indent);
            println!();
        },
        On{ location, block, .. } => {
            // Print the on + the location
            print!("{}on ", indent!(indent));
            pass_expr(location, indent);
            // Print the block
            print!(" ");
            pass_block(&*block, indent);
            println!();  
        },
        Parallel{ result, blocks, .. } => {
            // If there is a result, parse that first
            print!("{}", indent!(indent));
            if let Some(result) = result {
                print!("let ");
                pass_identifier(result);
                print!(" = ");
            }
            // Print the parallel thingy
            println!("parallel [");
            // Print the blocks
            for b in blocks {
                pass_stmt(&**b, indent + 3);
            }
            println!("{}]", indent!(indent));
        },

        LetAssign{ name, value, .. } => {
            // Print the let thingy first + the name
            print!("{}let ", indent!(indent));
            pass_identifier(name);
            // Print the expression
            print!(" := ");
            pass_expr(value, indent);
            println!(";");
        },
        Assign{ name, value, .. } => {
            // Just print the identifier
            print!("{}", indent!(indent));
            pass_identifier(name);
            // Print the expression
            print!(" := ");
            pass_expr(value, indent);
            println!(";");
        },
        Expr{ expr, .. } => {
            // Print the expression + semicolon
            print!("{}", indent!(indent));
            pass_expr(expr, indent);
            println!(";");
        },

        Empty{} => {},
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
pub fn pass_block(block: &Block, indent: usize) {
    // Print the curly bracket (no indent used, since it's expression position)
    println!("{{");
    // Print all statements with extra indent
    for s in block.stmts.iter() {
        pass_stmt(s, indent + INDENT_SIZE);
    }
    // Print the closing curly bracket
    print!("{}}}", indent!(indent));
}

/// Prints an Identifier node.
/// 
/// This will always be printed as a non-statement, so no indentation required.
/// 
/// # Arguments
/// - `identifier`: The Identifier to traverse.
/// 
/// # Returns
/// Nothing, but does print it.
pub fn pass_identifier(identifier: &Identifier) {
    // Print the full value
    print!("{}", identifier.value)
}

/// Prints a Property node.
/// 
/// # Arguments
/// - `prop`: The Property to traverse.
/// - `largest_prop`: The longest property name. It will auto-pad all names to this length to make a nice list.
/// - `indent`: The current base indent of all new lines to write.
/// 
/// # Returns
/// Nothing, but does print it.
pub fn pass_property(prop: &Property, largest_prop: usize, indent: usize) {
    // Print the identation, then the name identifier
    print!("{}", indent!(indent));
    pass_identifier(&prop.name);
    // Print the colon, then the data type
    println!("{} : {};", indent!(largest_prop - prop.name.value.len()), prop.data_type);
}

/// Prints an Expr node.
/// 
/// # Arguments
/// - `expr`: The Expr to traverse.
/// - `indent`: The current base indent of all new lines to write.
/// 
/// # Returns
/// Nothing, but does print it.
pub fn pass_expr(expr: &Expr, indent: usize) {
    // Match the expression
    use Expr::*;
    match expr {
        Cast{ expr, target, .. } => {
            // Print the cast operator
            print!("(({}) ", target);
            // Print the expression
            pass_expr(expr, indent);
            // Print the closing bracket
            print!(")");
        },

        Call{ expr, args, locations, .. } => {
            // Print the identifying expression
            pass_expr(expr, indent);
            // Print the arguments
            print!("(");
            let mut first = true;
            for a in args {
                if first { first = false; }
                else { print!(", "); }
                pass_expr(a, indent);
            }
            // Print the closing bracket
            print!(")");

            // Print locations
            if let AllowedLocations::Exclusive(locs) = locations {
                print!(" @[{}]", locs.iter().map(|l| l.into()).collect::<Vec<String>>().join(","));
            }
        },
        Array{ values, .. } => {
            // Print the values wrapped in '[]'
            print!("[");
            let mut first = true;
            for v in values {
                if first { first = false; }
                else { print!(", "); }
                pass_expr(v, indent);
            }
            print!("]");
        },
        ArrayIndex{ array, index, .. } => {
            // Print the array first
            pass_expr(array, indent);
            // Print the index expression wrapped in '[]'
            print!("[");
            pass_expr(index, indent);
            print!("]");
        },
        Pattern{ exprs, .. } => {
            // We use ad-hoc syntax for now
            print!("Pattern<");
            let mut first = true;
            for e in exprs {
                if first { first = false; }
                else { print!(", "); }
                pass_expr(e, indent);
            }
            print!(">");
        },

        UnaOp{ op, expr, .. } => {
            // How to print exact is operator-determined
            match op {
                dsl_ast::UnaOp::Idx{ .. } => {
                    // Print the expr expression wrapped in '[]'
                    print!("[");
                    pass_expr(expr, indent);
                    print!("]");
                },
                dsl_ast::UnaOp::Not{ .. } => {
                    // Print the logical negation, then the expression
                    print!("(!");
                    pass_expr(expr, indent);
                    print!(")");
                },
                dsl_ast::UnaOp::Neg{ .. } => {
                    // Print the mathmatical negation, then the expression
                    print!("(-");
                    pass_expr(expr, indent);
                    print!(")");
                },
                dsl_ast::UnaOp::Prio{ .. } => {
                    // Print simply in between brackets
                    print!("(");
                    pass_expr(expr, indent);
                    print!(")");
                },
            }
        },
        BinOp{ op, lhs, rhs, .. } => {
            // How to print exact is operator-determined
            match op {
                dsl_ast::BinOp::And{ .. } => {
                    // Print lhs && rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" && ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Or{ .. } => {
                    // Print lhs || rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" || ");
                    pass_expr(rhs, indent);
                    print!(")");
                },

                dsl_ast::BinOp::Add{ .. } => {
                    // Print lhs + rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" + ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Sub{ .. } => {
                    // Print lhs - rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" - ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Mul{ .. } => {
                    // Print lhs * rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" * ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Div{ .. } => {
                    // Print lhs / rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" / ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Mod{ .. } => {
                    // Print lhs % rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" % ");
                    pass_expr(rhs, indent);
                    print!(")");
                },

                dsl_ast::BinOp::Eq{ .. } => {
                    // Print lhs == rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" == ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Ne{ .. } => {
                    // Print lhs != rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" != ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Lt{ .. } => {
                    // Print lhs < rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" < ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Le{ .. } => {
                    // Print lhs <= rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" <= ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Gt{ .. } => {
                    // Print lhs > rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" > ");
                    pass_expr(rhs, indent);
                    print!(")");
                },
                dsl_ast::BinOp::Ge{ .. } => {
                    // Print lhs >= rhs
                    print!("(");
                    pass_expr(lhs, indent);
                    print!(" >= ");
                    pass_expr(rhs, indent);
                    print!(")");
                },

                // dsl_ast::BinOp::Proj{ .. } => {
                //     // Print lhs.rhs
                //     print!("(");
                //     pass_expr(lhs, indent);
                //     print!(".");
                //     pass_expr(rhs, indent);
                //     print!(")");
                // },
            }
        },
        Proj{ lhs, rhs, .. } => {
            // Print lhs.rhs
            pass_expr(lhs, indent);
            print!(".");
            pass_expr(rhs, indent);
        },

        Instance{ name, properties, .. } => {
            // Print 'new Name'
            print!("new ");
            pass_identifier(name);
            // Print the body
            println!(" {{");
            let largest_prop: usize = properties.iter().map(|p| p.name.value.len()).max().unwrap_or(0);
            for p in properties {
                // Print the proprerty name followed by its value
                pass_property_expr(p, largest_prop, indent + 3);
            }
            // Print the closing thingy
            print!("{}}}", indent!(indent));
        },
        VarRef{ name, .. } => {
            // Print the identifier
            pass_identifier(name);
        },
        Identifier{ name, .. } => {
            // Print the identifier
            pass_identifier(name);
        },
        Literal{ literal } => {
            // Print the literal
            pass_literal(literal);
        },

        Empty{} => {},
    }
}

/// Prints a PropertyExpr node.
/// 
/// # Arguments
/// - `prop`: The PropertyExpr to traverse.
/// - `largest_prop`: The longest property name. It will auto-pad all names to this length to make a nice list.
/// 
/// # Returns
/// Nothing, but does print it.
pub fn pass_property_expr(prop: &PropertyExpr, largest_prop: usize, indent: usize) {
    // Print the identation, then the name identifier
    print!("{}", indent!(indent));
    pass_identifier(&prop.name);
    // Print the colon, then the expression
    print!("{} : ", indent!(largest_prop - prop.name.value.len()));
    pass_expr(&prop.value, indent + 3);
    // Finally print the comma
    println!(",");
}

/// Prints a Literal node.
/// 
/// # Arguments
/// - `literal`: The Literal to traverse.
/// 
/// # Returns
/// Nothing, but does print it.
pub fn pass_literal(literal: &Literal) {
    // Match on the exact literal kind
    use Literal::*;
    match literal {
        Boolean{ value, .. } => {
            print!("{}", if *value { "true" } else { "false" });
        },
        Integer{ value, .. } => {
            print!("{}", *value);
        },
        Real{ value, .. } => {
            print!("{}", *value);
        },
        String{ value, .. } => {
            print!("\"{}\"", value);
        },
        Semver{ value, .. } => {
            print!("{}", value);
        }
        Void{ .. } => {
            print!("<void>");
        },
    }
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
/// This pass doesn't really error, but is here for convention purposes.
pub fn do_traversal(root: Program) -> Result<Program, Vec<Error>> {
    // Iterate over all statements and run the appropriate match
    for s in root.block.stmts.iter() {
        pass_stmt(s, 0);
    }

    // Done
    Ok(root)
}
