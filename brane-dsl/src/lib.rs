//  LIB.rs
//    by Lut99
// 
//  Created:
//    18 Aug 2022, 09:49:38
//  Last edited:
//    20 Oct 2022, 14:17:12
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines a library that converts BraneScript _or_ Bakery to a
//!   temporary AST. This AST may then be further analysed / compiled in
//!   the `brane-ast` crate.
// 

// Define private modules
mod scanner;
mod parser;

// Define public modules
pub mod errors;
pub mod spec;
pub mod data_type;
pub mod location;
pub mod symbol_table;
pub mod compiler;


// Bring some stuff into the crate namespace
pub use errors::ParseError as Error;
pub use spec::{Language, TextPos, TextRange};
pub use data_type::DataType;
pub use location::Location;
pub use symbol_table::SymbolTable;
pub use parser::ast;
pub use compiler::{parse, ParserOptions};


// Some useful, crate-local macros
#[cfg(any(feature = "print_parser_path", feature = "print_scanner_path"))]
thread_local!(
    /// A feature-dependent definition of a static variable indicating the parser nesting.
    static PARSER_PATH_NESTING: std::cell::RefCell<usize> = std::cell::RefCell::new(0);
);

/// A macro that can be used to enter some parser function.
#[allow(unused_macros)]
#[cfg(any(feature = "print_parser_path", feature = "print_scanner_path"))]
macro_rules! enter_pp {
    ($($arg:tt)+) => {
        {
            // Print the nesting
            print!("{} >> ", (0..crate::PARSER_PATH_NESTING.with(|v| *v.borrow())).map(|_| ' ').collect::<String>());
            // Print the rest
            println!($($arg)+);
            // Increment the parser path indent
            crate::PARSER_PATH_NESTING.with(|v| *v.borrow_mut() += 1);
        }
    };
}
#[allow(unused_macros)]
#[cfg(not(any(feature = "print_parser_path", feature = "print_scanner_path")))]
macro_rules! enter_pp {
    ($($arg:tt)+) => {};
}
#[allow(unused_imports)]
pub(crate) use enter_pp;

/// A macro that can be used to exit some parser function.
#[allow(unused_macros)]
#[cfg(any(feature = "print_parser_path", feature = "print_scanner_path"))]
macro_rules! exit_pp {
    ($res:expr, $($arg:tt)+) => {
        {
            // Decrement the parser path indent
            crate::PARSER_PATH_NESTING.with(|v| *v.borrow_mut() -= 1);
            // Print the nesting
            print!("{} << ", (0..crate::PARSER_PATH_NESTING.with(|v| *v.borrow())).map(|_| ' ').collect::<String>());
            // Print the rest
            println!($($arg)+);
            // Return the given value
            $res
        }
    };
}
#[allow(unused_macros)]
#[cfg(not(any(feature = "print_parser_path", feature = "print_scanner_path")))]
macro_rules! exit_pp {
    ($res:expr, $($arg:tt)+) => { $res };
}
#[allow(unused_imports)]
pub(crate) use exit_pp;

/// A macro that returns the given expression with proper `enter_pp!` and `exit_pp!` calls.
#[allow(unused_macros)]
#[cfg(any(feature = "print_parser_path", feature = "print_scanner_path"))]
macro_rules! wrap_pp {
    ($res:expr, $($arg:tt)+) => {
        {
            // Run it
            crate::enter_pp!($($arg)+);
            crate::exit_pp!($res, $($arg)+)
        }
    };
}
#[allow(unused_macros)]
#[cfg(not(any(feature = "print_parser_path", feature = "print_scanner_path")))]
macro_rules! wrap_pp {
    ($res:expr, $($arg:tt)+) => { $res };
}
#[allow(unused_imports)]
pub(crate) use wrap_pp;
