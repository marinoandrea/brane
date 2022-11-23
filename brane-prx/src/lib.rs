//  LIB.rs
//    by Lut99
// 
//  Created:
//    23 Nov 2022, 10:34:23
//  Last edited:
//    23 Nov 2022, 11:29:46
//  Auto updated?
//    Yes
// 
//  Description:
//!   The `brane-prx` crate implements a proxy service that maps incoming
//!   traffic on one port to a destination on the other. It basically does
//!   a man-in-the-middle attack lel.
// 

// Declare modules
pub mod errors;
pub mod spec;
pub mod ports;
pub mod manage;
pub mod redirect;
