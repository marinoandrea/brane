//  LOCATION.rs
//    by Lut99
// 
//  Created:
//    05 Sep 2022, 16:27:08
//  Last edited:
//    14 Nov 2022, 11:07:37
//  Auto updated?
//    Yes
// 
//  Description:
//!   Resolves the extra location restrictions that on-structures impose.
//! 
//!   Note that this traversal is actually only here in a deprecated fashion.
// 

use std::collections::HashSet;

use brane_dsl::TextRange;
use brane_dsl::location::{AllowedLocations, Location};
use brane_dsl::ast::{Block, Expr, Literal, Node, Program, Stmt};

pub use crate::errors::LocationError as Error;
use crate::errors::AstError;


/***** TESTS *****/
#[cfg(test)]
mod tests {
    use brane_dsl::ParserOptions;
    use brane_shr::utilities::{create_data_index, create_package_index, test_on_dsl_files};
    use specifications::data::DataIndex;
    use specifications::package::PackageIndex;
    use super::*;
    use super::super::print::dsl;
    use crate::{compile_program_to, CompileResult, CompileStage};


    /// Tests the traversal by generating symbol tables for every file.
    #[test]
    fn test_location() {
        test_on_dsl_files("BraneScript", |path, code| {
            // Start by the name to always know which file this is
            println!("{}", (0..80).map(|_| '-').collect::<String>());
            println!("File '{}' gave us:", path.display());

            // Load the package index
            let pindex: PackageIndex = create_package_index();
            let dindex: DataIndex    = create_data_index();

            // Run up to this traversal
            let program: Program = match compile_program_to(code.as_bytes(), &pindex, &dindex, &ParserOptions::bscript(), CompileStage::Location) {
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
                    panic!("Failed to analyse locations (see output above)");
                }
                CompileResult::Err(errs) => {
                    // Print the errors
                    for e in errs {
                        e.prettyprint(path.to_string_lossy(), &code);
                    }
                    panic!("Failed to analyse locations (see output above)");
                },

                _ => { unreachable!(); },
            };

            // Now print the file for prettyness
            dsl::do_traversal(program).unwrap();
            println!("{}\n\n", (0..80).map(|_| '-').collect::<String>());
        });
    }
}





/***** TRAVERSAL FUNCTIONS *****/
/// Attempts to resolve the location restrictions of all function calls in this Stmt.
/// 
/// # Arguments
/// - `stmt`: The Stmt to traverse.
/// - `locations`: The current restriction of locations as imposed by the on-structs.
/// - `reasons`: The ranges of the on-structs that somehow restrict the current call.
/// - `errors`: A list we use to accumulate errors as they occur.
/// 
/// # Errors
/// This function may error if there were semantic problems while resolving the locations.
/// 
/// If errors occur, they are appended to the `errors` list. The function is early-quit in that case.
fn pass_stmt(stmt: &mut Stmt, locations: AllowedLocations, reasons: Vec<TextRange>, errors: &mut Vec<Error>) {
    // Match on the exact statement
    use Stmt::*;
    #[allow(clippy::collapsible_match)]
    match stmt {
        Block{ block, .. } => {
            pass_block(block, locations, reasons, errors);
        },

        FuncDef{ code, .. } => {
            pass_block(code, locations, reasons, errors);
        },
        ClassDef{ methods, .. } => {
            for m in methods {
                pass_stmt(m, locations.clone(), reasons.clone(), errors);
            }
        },
        Return{ expr, .. } => {
            if let Some(expr) = expr { pass_expr(expr, locations, reasons, errors); }
        },

        If{ cond, consequent, alternative, .. } => {
            pass_expr(cond, locations.clone(), reasons.clone(), errors);
            pass_block(consequent, locations.clone(), reasons.clone(), errors);
            if let Some(alternative) = alternative { pass_block(alternative, locations, reasons, errors) };
        },
        For{ initializer, condition, increment, consequent, .. } => {
            pass_stmt(initializer, locations.clone(), reasons.clone(), errors);
            pass_expr(condition, locations.clone(), reasons.clone(), errors);
            pass_stmt(increment, locations.clone(), reasons.clone(), errors);
            pass_block(consequent, locations, reasons, errors);
        },
        While{ condition, consequent, .. } => {
            pass_expr(condition, locations.clone(), reasons.clone(), errors);
            pass_block(consequent, locations, reasons, errors);
        },
        On{ location, block, range, .. } => {
            // Enfore the location to be a string constant (we do always expect a cast due to type analysis).
            let loc: String = if let brane_dsl::ast::Expr::Cast { expr, .. } = location {
                if let brane_dsl::ast::Expr::Literal { literal: Literal::String{ value, .. } } = &**expr {
                    value.clone()
                } else {
                    errors.push(Error::IllegalLocation { range: location.range().clone() });
                    return;
                }
            } else {
                errors.push(Error::IllegalLocation { range: location.range().clone() });
                return;
            };

            // See what this additional restriction imposes
            let mut locations: AllowedLocations = locations;
            locations.intersection(&mut AllowedLocations::Exclusive(HashSet::from([ Location::from(loc) ])));
            if locations.is_empty() {
                errors.push(Error::OnNoLocation { range: range.clone(), reasons });
                return;
            }

            // With the new restrictions set, recurse
            let mut reasons: Vec<TextRange> = reasons;
            reasons.push(range.clone());
            pass_block(block, locations, reasons, errors);
        },
        Parallel{ blocks, .. } => {
            for b in blocks {
                pass_stmt(b, locations.clone(), reasons.clone(), errors);
            }
        },

        LetAssign{ value, .. } => {
            pass_expr(value, locations, reasons, errors);
        },
        Assign{ value, .. } => {
            pass_expr(value, locations, reasons, errors);
        },
        Expr{ expr, .. } => {
            pass_expr(expr, locations, reasons, errors);
        },

        // The rest no matter
        _ => {},
    };
}

/// Attempts to resolve the location restrictions of all function calls in this Block.
/// 
/// # Arguments
/// - `block`: The Block to traverse.
/// - `locations`: The current restriction of locations as imposed by the on-structs.
/// - `reasons`: The ranges of the on-structs that somehow restrict the current call.
/// - `errors`: A list we use to accumulate errors as they occur.
/// 
/// # Errors
/// This function may error if there were semantic problems while resolving the locations.
/// 
/// If errors occur, they are appended to the `errors` list. The function is early-quit in that case.
fn pass_block(block: &mut Block, locations: AllowedLocations, reasons: Vec<TextRange>, errors: &mut Vec<Error>) {
    // Simply recurse
    for s in &mut block.stmts {
        pass_stmt(s, locations.clone(), reasons.clone(), errors);
    }
}

/// Attempts to resolve the location restrictions of all function calls in this Expr.
/// 
/// # Arguments
/// - `expr`: The Expr to traverse.
/// - `on_locations`: The current restriction of locations as imposed by the on-structs.
/// - `on_reasons`: The ranges of the on-structs that somehow restrict the current call.
/// - `errors`: A list we use to accumulate errors as they occur.
/// 
/// # Returns
/// This function returns the restrictions of the expression as a whole, together with a list of sources for that restriction. This only applies to calls within it, but is necessary for parent calls to know about.
/// 
/// # Errors
/// This function may error if there were semantic problems while resolving the locations.
/// 
/// If errors occur, they are appended to the `errors` list. The function is early-quit in that case.
fn pass_expr(expr: &mut Expr, on_locations: AllowedLocations, on_reasons: Vec<TextRange>, errors: &mut Vec<Error>) {
    use Expr::*;
    match expr {
        Cast{ expr, .. } => {
            pass_expr(expr, on_locations, on_reasons, errors);
        },

        Call{ expr, args, ref mut locations, range, .. } => {
            // Resolve the nested stuff first
            pass_expr(expr, on_locations.clone(), on_reasons.clone(), errors);
            for a in args {
                pass_expr(a, on_locations.clone(), on_reasons.clone(), errors);
            }

            // Add the current location if it added to the restriction
            let mut on_reasons: Vec<TextRange> = on_reasons;
            if locations.is_exclusive() { on_reasons.push(range.clone()); }

            // Take the union of the already imposed restrictions + those imposed by On-blocks
            let mut on_locations: AllowedLocations = on_locations;
            locations.intersection(&mut on_locations);
            if locations.is_empty() { errors.push(Error::NoLocation { range: range.clone(), reasons: on_reasons }); }
        },
        Array{ values, .. } => {
            for v in values {
                pass_expr(v, on_locations.clone(), on_reasons.clone(), errors);
            }
        },
        ArrayIndex{ array, index, .. } => {
            pass_expr(array, on_locations.clone(), on_reasons.clone(), errors);
            pass_expr(index, on_locations, on_reasons, errors);
        },

        UnaOp{ expr, .. } => {
            pass_expr(expr, on_locations, on_reasons, errors);
        },
        BinOp{ lhs, rhs, .. } => {
            pass_expr(lhs, on_locations.clone(), on_reasons.clone(), errors);
            pass_expr(rhs, on_locations, on_reasons, errors);
        },
        Proj{ lhs, rhs, .. } => {
            pass_expr(lhs, on_locations.clone(), on_reasons.clone(), errors);
            pass_expr(rhs, on_locations, on_reasons, errors);
        },

        Instance{ properties, .. } => {
            for p in properties {
                pass_expr(&mut p.value, on_locations.clone(), on_reasons.clone(), errors);
            }
        },

        // The rest we don't care
        _ => {},
    }
}





/***** LIBRARY *****/
/// Resolves typing in the given `brane-dsl` AST.
/// 
/// Note that the symbol tables must already have been constructed.
/// 
/// This effectively resolves all unresolved types in the symbol tables and verifies everything is compatible. Additionally, it may also insert implicit type casts where able.
/// 
/// # Arguments
/// - `root`: The root node of the tree on which this compiler pass will be done.
/// 
/// # Returns
/// The same nodes as went in, but now with no unresolved types.
/// 
/// # Errors
/// This pass may throw multiple `AstError::ResolveError`s if the user made mistakes with their variable references.
pub fn do_traversal(root: Program) -> Result<Program, Vec<AstError>> {
    let mut root = root;

    // Iterate over all statements to build their symbol tables (if relevant)
    let mut errors: Vec<Error> = vec![];
    for s in root.block.stmts.iter_mut() {
        pass_stmt(s, AllowedLocations::All, vec![], &mut errors);
    }

    // Done
    if errors.is_empty() {
        Ok(root)
    } else {
        Err(errors.into_iter().map(|e| e.into()).collect())
    }
}
