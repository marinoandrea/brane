//  MOD.rs
//    by Lut99
// 
//  Created:
//    25 Aug 2022, 11:01:39
//  Last edited:
//    25 Aug 2022, 11:17:25
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines the module that implements the scanner part of the parser.
// 

// Declare the modules
pub mod tokens;
pub mod comments;
pub mod literal;
pub mod scanner;

// Bring some stuff into this namespace
pub use tokens::{Token, Tokens};
pub use scanner::scan_tokens;


// Define some useful types for this module
/// Shortcut for a LocatedSpan that carries a &str and no additional data.
pub type Span<'a> = nom_locate::LocatedSpan<&'a str>;


// Define some useful macros for this module
/// Wrapper around wrap_pp that is only enabled if print_scanner_path is enabled
#[cfg(feature = "print_scanner_path")]
macro_rules! wrap_pp {
    ($res:expr, $($arg:tt)+) => {
        crate::wrap_pp!($res, $($arg)+)
    };
}
/// Wrapper around wrap_pp that is only enabled if print_scanner_path is enabled
#[cfg(not(feature = "print_scanner_path"))]
macro_rules! wrap_pp {
    ($res:expr, $($arg:tt)+) => {
        $res
    };
}
pub(crate) use wrap_pp;
