//  COMPILE.rs
//    by Lut99
// 
//  Created:
//    16 Nov 2022, 14:36:38
//  Last edited:
//    16 Nov 2022, 16:49:49
//  Auto updated?
//    Yes
// 
//  Description:
//!   Handles things related to offline compiling.
// 

use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use brane_dsl::Language;
use brane_ast::{compile_program, CompileResult, ParserOptions, Workflow};
use specifications::data::DataIndex;
use specifications::package::PackageIndex;
use specifications::registry::RegistryConfig;

pub use crate::errors::CompileError as Error;
use crate::utils::get_registry_file;
use crate::data;
use crate::packages;


/***** LIBRARY *****/
/// Compiles the given BraneScript file to a full Workflow.
/// 
/// # Arguments
/// - `language`: The language to compile.
/// - `input`: The file that is the input. Can also be '-' to read from stdin instead.
/// - `output`: The file to output to. Can also be '-' to write to stdout.
/// - `compact`: If given, serializes with as little whitespace as possible. Decreases the resulting size greatly, but also readability.
/// - `remote`: Whether to use the local package- and data index (false) or a remote one (true).
/// 
/// # Returns
/// Nothing, but does write the compiled workflow to the given output file.
/// 
/// # Errors
/// This function errors if the input is not valid BraneScript.
pub async fn compile(language: Language, input: impl AsRef<Path>, output: impl AsRef<Path>, compact: bool, remote: bool) -> Result<(), Error> {
    let input  : &Path = input.as_ref();
    let output : &Path = output.as_ref();

    // Open the given input
    let (name, source): (Cow<str>, String) = {
        let (name, mut handle): (Cow<str>, Box<dyn Read>) = if input != PathBuf::from("-") {
            debug!("Opening input file '{}'...", input.display());
            match File::open(&input) {
                Ok(handle) => (Cow::from(input.display().to_string()), Box::new(handle)),
                Err(err)   => { return Err(Error::InputOpenError{ path: input.into(), err }); },
            }
        } else {
            ("<stdin>".into(), Box::new(std::io::stdin()))
        };

        // Read it
        debug!("Reading from '{}'...", name);
        let mut source: String = String::new();
        if let Err(err) = handle.read_to_string(&mut source) {
            return Err(Error::InputReadError{ name: name.to_string(), err });
        }

        // Done
        (name, source)
    };

    // Fetch the indices
    let (pindex, dindex): (PackageIndex, DataIndex) = if remote {
        debug!("Fetching remote indices...");

        // Fetch the address
        let config: RegistryConfig = match get_registry_file() {
            Ok(config) => config,
            Err(err)   => { return Err(Error::RegistryConfigError{ err }); },
        };

        // Fetch the things themselves
        (
            match brane_tsk::api::get_package_index(&config.url).await {
                Ok(pindex) => pindex,
                Err(err)   => { return Err(Error::RemotePackageIndexError { endpoint: config.url, err }); },
            },
            match brane_tsk::api::get_data_index(&config.url).await {
                Ok(dindex) => dindex,
                Err(err)   => { return Err(Error::RemoteDataIndexError { endpoint: config.url, err }); },
            },
        )
    } else {
        debug!("Fetching local indices...");

        // Fetch the indices
        (
            match packages::get_package_index() {
                Ok(pindex) => pindex,
                Err(err)   => { return Err(Error::LocalPackageIndexError { err }); },
            },
            match data::get_data_index() {
                Ok(dindex) => dindex,
                Err(err)   => { return Err(Error::LocalDataIndexError { err }); },
            },
        )
    };

    // Compile it
    debug!("Compiling workflow...");
    let workflow: Workflow = match compile_program(source.as_bytes(), &pindex, &dindex, &ParserOptions::new(language)) {
        CompileResult::Workflow(workflow, warns) => {
            // Print any warnings (on stderr)
            for warn in warns {
                warn.prettyprint(&name, &source);
            }

            // Return the workflow
            workflow
        },
        CompileResult::Unresolved(_, _) => unreachable!(),
        CompileResult::Program(_, _)    => unreachable!(),
        CompileResult::Eof(err) => {
            err.prettyprint(name, source);
            return Err(Error::CompileError{ errs: vec![ err ] });
        },
        CompileResult::Err(errs) => {
            for err in &errs {
                err.prettyprint(&name, &source);
            }
            return Err(Error::CompileError{ errs });
        },
    };

    // Serialize the output
    let sworkflow: String = if !compact {
        match serde_json::to_string_pretty(&workflow) {
            Ok(sworkflow) => sworkflow,
            Err(err)      => { return Err(Error::WorkflowSerializeError{ err }); },
        }
    } else {
        match serde_json::to_string(&workflow) {
            Ok(sworkflow) => sworkflow,
            Err(err)      => { return Err(Error::WorkflowSerializeError{ err }); },
        }
    };

    // Write to the given output
    {
        // Open it
        let (name, mut handle): (Cow<str>, Box<dyn Write>) = if output != PathBuf::from("-") {
            debug!("Creating output file '{}'...", output.display());
            match File::create(&output) {
                Ok(handle) => (output.display().to_string().into(), Box::new(handle)),
                Err(err)   => { return Err(Error::OutputCreateError{ path: output.into(), err }); },
            }
        } else {
            ("<stdout>".into(), Box::new(std::io::stdout()))
        };

        // Write it
        if let Err(err) = handle.write_all(sworkflow.as_bytes()) { return Err(Error::OutputWriteError { name: name.into(), err }); }

        // Bob it, twist it
    }

    // Done
    Ok(())
}
