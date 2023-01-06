//  LIB.rs
//    by Lut99
// 
//  Created:
//    18 Nov 2022, 14:37:19
//  Last edited:
//    18 Nov 2022, 15:05:28
//  Auto updated?
//    Yes
// 
//  Description:
//!   The `branec` executable implements a CLI-compatible version of the
//!   BraneScript / Bakery compiler.
//!   
//!   Specifically, it features options to compile certain source files to
//!   usable JSON, and it (will eventually) host options to parse & generate
//!   BraneScript's assembly (showing the instructions and junk).
// 

// Declare modules
pub mod errors;
pub mod spec;
