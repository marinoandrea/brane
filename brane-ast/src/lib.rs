//  LIB.rs
//    by Lut99
// 
//  Created:
//    10 Aug 2022, 13:51:38
//  Last edited:
//    16 Sep 2022, 08:23:34
//  Auto updated?
//    Yes
// 
//  Description:
//!   The `brane-ast` package takes a parsed AST and converts it to one
//!   that is runnable. Specifically, it implements multiple compiler
//!   passes that resolve different things (such as type-safety or data
//!   ownership).
// 

// Use macros
#[macro_use]
extern crate lazy_static;

// Declare the modules
pub mod errors;
pub mod warnings;
pub mod spec;
pub mod data_type;
pub mod locations;
pub mod ast;
pub mod edgebuffer;
pub mod ast_unresolved;
pub mod state;
pub mod traversals;
pub mod compile;
pub mod fetcher;


// Re-export some stuff from brane-dsl
pub use brane_dsl::ParserOptions;
pub use brane_dsl::spec::{MergeStrategy, TextPos, TextRange};

// Bring some stuff into the global namespace.
pub use errors::AstError as Error;
pub use warnings::AstWarning as Warning;
pub use data_type::DataType;
pub use ast::{SymTable, Workflow};
pub use ast_unresolved::UnresolvedWorkflow;
pub use compile::{compile_program, compile_program_to, compile_snippet, compile_snippet_to, CompileResult, CompileStage};
