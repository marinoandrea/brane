//  LIB.rs
//    by Lut99
// 
//  Created:
//    26 Sep 2022, 12:00:46
//  Last edited:
//    28 Nov 2022, 16:09:14
//  Auto updated?
//    Yes
// 
//  Description:
//!   The `brane-drv` crate implements the 'user delegate' in the central
//!   compute node. To be more precise, it takes user workflows and runs
//!   them, scheduling and orchestrating external function calls (tasks)
//!   where necessary.
// 

// Declare the modules
pub mod errors;
pub mod spec;
pub mod planner;
pub mod vm;
pub mod handler;
