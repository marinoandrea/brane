//  BUILD.rs
//    by Lut99
// 
//  Created:
//    27 Oct 2022, 10:28:41
//  Last edited:
//    29 Nov 2022, 13:07:50
//  Auto updated?
//    Yes
// 
//  Description:
//!   Build script for the `brane-tsk` crate. Basically just compiles the
//!   `.proto` file(s) to Rust.
// 


/***** ENTRYPOINT *****/
fn main() -> Result<(), std::io::Error> {
    tonic_build::configure()
        .compile(&["proto/driver.proto", "proto/job.proto"], &["proto"])
}
