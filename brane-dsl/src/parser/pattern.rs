//  PATTERN.rs
//    by Lut99
// 
//  Created:
//    17 Aug 2022, 11:00:13
//  Last edited:
//    06 Sep 2022, 15:27:41
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains compiler functions for compiling patterns to function
//!   calls.
// 

use std::iter;

use itertools::interleave;
use rand::distributions::Alphanumeric;
use rand::Rng;
use regex::Regex;

use specifications::common::{CallPattern, Function, Parameter};
use specifications::package::{PackageKind, PackageIndex, PackageInfo};

pub use crate::errors::PatternError as Error;
use crate::location::AllowedLocations;
use crate::spec::TextRange;
use crate::parser::ast::{Expr, Identifier, Stmt};


/***** HELPER TYPES *****/
/// Shortcut for a HashMap with string keys.
type Map<T> = std::collections::HashMap<String, T>;




/***** HELPER STRUCTS *****/
#[derive(Clone, Debug)]
pub struct FunctionPattern {
    pub parameters: Vec<Parameter>,
    pub name: String,
    pub meta: Map<String>,
    pub pattern: String,
    pub return_type: String,
}




/***** HELPER FUNCTIONS *****/
/// Converts a given Pattern expression to a Call expressions.
/// 
/// # Arguments
/// - `pattern`: The expressions contained within the pattern expression.
/// - `range`: The start & end position of the pattern in the original source text.
/// - `patterns`: The function patterns as defined in the packages.
/// 
/// # Returns
/// A new `Expr::Call` that represents this pattern but classically.
/// 
/// # Errors
/// This function may error if the pattern was semantically invalid.
fn pattern_to_call(
    pattern: Vec<Box<Expr>>,
    range: TextRange,
    patterns: &[FunctionPattern],
) -> Result<Expr, Error> {
    // Rewrite the pattern as a string
    let terms_pattern = build_terms_pattern(&pattern);
    debug!("Attempting to rewrite to call: {:?}", terms_pattern);

    // Match it to a function
    let (function, indexes) = match_pattern_to_function(terms_pattern, range.clone(), patterns)?;
    let arguments = indexes.into_iter().map(|i| pattern.get(i).unwrap()).cloned().collect();

    Ok(Expr::new_call(
        Box::new(Expr::new_identifier(Identifier::new(function.name, TextRange::none()))),
        arguments,

        range,
        AllowedLocations::All,
    ))
}

/// Builds a function pattern (i.e., a regex expression) from the given Function definition.
/// 
/// # Arguments
/// - `name`: The name of the function.
/// - `function`: The Function to create a pattern for.
/// 
/// # Returns
/// A new String containing the regex expression that represents this pattern.
fn build_pattern(
    name: &str,
    function: &Function,
) -> String {
    // Prepare the pattern vector with enough space
    let mut pattern: Vec<String> = Vec::with_capacity(1 + 1 + function.parameters.len() + 1);
    // Extract the pattern notation from the function
    let notation = function.pattern.clone().unwrap_or(CallPattern::new(None, None, None));
    // Collect the arguments of the function as regex expressions expecting that type
    let mut arguments: Vec<String> = function
        .parameters
        .iter()
        .filter(|p| p.secret.is_none()) // Ignore implicit arguments
        .map(|arg| {
            let data_type = regex::escape(&arg.data_type);
            let data_type = if data_type.ends_with(']') {
                format!("{}|array", data_type)
            } else if data_type.chars().next().unwrap().is_uppercase() {
                format!("{}|object", data_type)
            } else {
                data_type
            };

            format!("<[\\.\\w]+:({})>", data_type)
        })
        .collect();

    // Either put the prefix there or the function name if there is no pattern defined
    if let Some(prefix) = notation.prefix {
        pattern.push(regex::escape(&prefix));
    } else {
        pattern.push(regex::escape(name));
    }

    // Insert the infix in between all arguments if present, then add all arguments
    if let Some(infix) = notation.infix {
        let infix: Vec<String> = infix.iter().map(|i| regex::escape(i)).collect();
        arguments = interleave(arguments, infix).collect();
    }
    for argument in arguments {
        pattern.push(argument);
    }

    // If there is a postfix, add that
    if let Some(postfix) = notation.postfix {
        pattern.push(regex::escape(&postfix));
    }

    // Return as a space-separated string
    pattern.join(" ")
}

/// Rebuild the pattern as a large string, with some additional information (such as type information for literals) inserted into it.
/// 
/// # Arguments
/// - `terms`: The terms (expressions) in a pattern expression that we will rewrite to a string.
/// 
/// # Returns
/// A string with the same expression.
fn build_terms_pattern(terms: &[Box<Expr>]) -> String {
    // Go through the terms to serialize them
    let mut term_pattern_segments: Vec<String> = Vec::with_capacity(terms.len());
    for term in terms {
        match &**term {
            Expr::VarRef{ name: id, .. } => {
                term_pattern_segments.push(id.value.clone());
            },

            Expr::Literal{ literal, .. } => {
                let temp_var = create_temp_var(true);
                let segment = format!("<{}:{}>", temp_var, literal.data_type());

                term_pattern_segments.push(segment);
            },

            _ => unreachable!(),
        }
    }

    // Put it in a space-separated list and done is cees
    term_pattern_segments.join(" ")
}

/// Creates a temporary, random variable identifier.
/// 
/// # Arguments
/// - `literal`: Whether this temporary variable will be for a literal or not (if not, the name will be preceded by an underscore).
/// 
/// # Returns
/// The identifier of the new random variable.
fn create_temp_var(literal: bool) -> String {
    // Simply create an identifier of 5 random alphanumeric characters
    let mut rng = rand::thread_rng();
    let identifier: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(5)
        .collect();

    // Return normal or with an underscore, depending on `literal`.
    if literal {
        identifier
    } else {
        format!("_{}", identifier)
    }
}

/// Matches a given string pattern to a FunctionPattern.
/// 
/// # Arguments
/// - `pattern`: The (string) pattern to match.
/// - `range`: The start & end position of the pattern in the original source text.
/// - `functions`: The list of FunctionPatterns that may match the given pattern.
/// 
/// # Returns
/// The function pattern that is matched, together with the indices of the arguments in the pattern that are passed in the call.
/// 
/// # Errors
/// This function may error if no pattern was matched.
fn match_pattern_to_function(
    pattern: String,
    range: TextRange,
    functions: &[FunctionPattern],
) -> Result<(FunctionPattern, Vec<usize>), Error> {
    // Iterate over all possible patterns
    for function in functions {
        // The pattern is a regex, so parse it as such
        debug!("Check: {:?}", &function.pattern);
        let reg = Regex::new(&function.pattern).unwrap();

        // If the regex matches, it's our pattern
        if let Some(coverage) = reg.find(&pattern) {
            // We must have matched the entire pattern for it to match
            if coverage.start() == 0 && coverage.end() == pattern.len() {
                debug!("match: {:?}", &function.pattern);

                // Collect the indices of the split values in the pattern that form the arguments to the call
                let arg_indexes: Vec<usize> = pattern
                    .split(' ')
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, t)| if t.starts_with('<') { Some(i) } else { None })
                    .collect();

                // Return the found match
                return Ok((function.clone(), arg_indexes));
            }
        }
    }

    // Jep no pattern found
    Err(Error::UnknownPattern{ raw: pattern, range })
}





/***** LIBRARY *****/
/// Function that resolves patterns to function calls so they may be ignored for subsequent compiler passes.
/// 
/// This function is still quire rudimentary.
/// 
/// # Arguments
/// - `program`: The program of statements that might contain patterns.
/// - `package_index`: The PackageIndex used to resolve pattern calls.
/// 
/// # Returns
/// The same vector with statements, but now with patterns replaced in them.
/// 
/// # Errors
/// This function errors if the pattern was invalid somehow.
pub fn resolve_patterns(
    program: Vec<Stmt>,
    package_index: &PackageIndex,
) -> Result<Vec<Stmt>, Error> {
    let mut program: Vec<Stmt> = program;

    // Read the function patterns that are defined in the packages
    let function_patterns: Vec<FunctionPattern> = package_index
        .packages
        .values()
        .flat_map(|p| get_module_patterns(p))
        .collect();

    // Convert all pattern expressions this way
    for i in 0..program.len() {
        // Check if this statement is a pattern expression
        if let Stmt::Expr{ expr: Expr::Pattern{ exprs, range: prange }, range, .. } = &program[i] {
            // Replace it
            program[i] = Stmt::new_expr(pattern_to_call(exprs.clone(), prange.clone(), &function_patterns)?, range.clone());
        }
    }

    // Return the processed program
    Ok(program)
}



/// Given a package (as a PackageInfo), returns a list of FunctionPatterns that may be used to convert pattern expressions to full function calls.
/// 
/// # Arugments
/// - `module`: The module to extract the patterns from.
/// 
/// # Returns
/// A list of all FunctionPatterns found in the PackageInfo.
pub fn get_module_patterns(module: &PackageInfo) -> Vec<FunctionPattern> {
    // Simply iterate over all of the functions and create a pattern for each of them
    let mut patterns: Vec<FunctionPattern> = Vec::with_capacity(module.functions.len());
    for (name, function) in module.functions.iter() {
        // Build the pattern for this function
        let pattern = build_pattern(name, function);

        // Build a map with metadata for this package (?)
        let mut meta = Map::<String>::new();
        meta.insert(String::from("kind"), String::from(module.kind));
        meta.insert(String::from("name"), module.name.clone());
        meta.insert(String::from("version"), module.version.to_string());
        if module.kind != PackageKind::Dsl {
            meta.insert(String::from("image"), format!("{}:{}", module.name, module.version));
        }

        // Put that in a FunctionPattern.
        let function_pattern = FunctionPattern {
            parameters: function.parameters.clone(),
            meta,
            name: name.clone(),
            pattern,
            return_type: function.return_type.clone(),
        };

        patterns.push(function_pattern);
    }
    patterns
}
