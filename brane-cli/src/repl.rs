use crate::{packages, registry};
use anyhow::Result;
use brane_dsl::compiler::{Compiler, CompilerOptions};
use brane_vm::sync_machine::SyncMachine;
use brane_vm::environment::InMemoryEnvironment;
use brane_vm::vault::InMemoryVault;
use brane_sys::local::LocalSystem;
use futures::executor::block_on;
use linefeed::{Interface, ReadResult};
use specifications::common::Value;
use specifications::instructions::Instruction;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use serde_yaml;
use uuid::Uuid;
use serde_json::json;
use console::style;

type Map<T> = std::collections::HashMap<String, T>;

pub async fn start(
    secrets_file: Option<PathBuf>,
    co_address: Option<PathBuf>,
) -> Result<()> {
    if let Some(co_address) = co_address {
        return start_compile_service(co_address).await;
    }

    println!("Starting interactive session, press Ctrl+D to exit.\n");

    let interface = Interface::new("brane-repl")?;
    interface.set_prompt("brane> ")?;

    // Prepare DSL compiler
    let package_index = registry::get_package_index().await?;
    let mut compiler = Compiler::new(CompilerOptions::repl(), package_index)?;

    // Prepare machine
    let secrets: Map<Value> = if let Some(secrets_file) = secrets_file {
        let reader = BufReader::new(File::open(&secrets_file)?);
        serde_yaml::from_reader(reader)?
    } else {
        Default::default()
    };

    let session_id = Uuid::new_v4();

    let environment = InMemoryEnvironment::new(None, None);
    let system = LocalSystem::new(session_id);
    let vault = InMemoryVault::new(secrets);

    let mut machine = SyncMachine::new(
        Box::new(environment),
        Box::new(system),
        Box::new(vault),
    );

    while let ReadResult::Input(line) = interface.read_line()? {
        if !line.trim().is_empty() {
            interface.add_history_unique(line.clone());
        };

        let instructions = compiler.compile(&line);
        debug!("Instructions: {:?}", instructions);

        match instructions {
            Ok(instructions) => {
                let instructions = preprocess_instructions(&instructions)?;
                let output = machine.walk(&instructions)?;

                match output {
                    Value::Unit => {},
                    _ => print(&output, false),
                }
            }
            Err(err) => {
                error!("{:?}", err);
            }
        }
    }

    println!("Goodbye.");
    Ok(())
}

///
///
///
async fn start_compile_service(co_address: PathBuf) -> Result<()> {
    println!("Starting compile-only service\n");

    // Prepare DSL compiler
    let package_index = registry::get_package_index().await?;
    let mut compiler = Compiler::new(CompilerOptions::repl(), package_index)?;

    let context = zmq::Context::new();
    let socket = context.socket(zmq::REP)?;

    let address = co_address.into_os_string().into_string().unwrap();
    let endpoint = format!("ipc://{}", address);

    debug!("endpoint: {}", endpoint);
    socket.bind(&endpoint)?;

    loop {
        let data = socket.recv_string(0)?;

        if let Ok(data) = data {
            let result = compiler.compile(&data);
            let response = match result {
                Ok(instructions) => {
                    json!({
                        "variant": "ok",
                        "content": instructions,
                    })
                },
                Err(err) => {
                    debug!("{}", err);
                    json!({
                        "variant": "err",
                        "content": err.to_string(),
                    })
                }
            };

            let response_json = serde_json::to_string(&response)?;
            debug!("Response: {}", response_json);

            socket.send(&response_json, 0)?;
        } else {
            warn!("Failed to read compile-service request, ignoring..");
            socket.send("", 0)?;
        };
    }    
}

///
///
///
fn preprocess_instructions(instructions: &Vec<Instruction>) -> Result<Vec<Instruction>> {
    let mut instructions = instructions.clone();

    for instruction in &mut instructions {
        match instruction {
            Instruction::Act(act) => {
                let name = act.meta.get("name").expect("No `name` property in metadata.");
                let version = act.meta.get("version").expect("No `version` property in metadata.");
                let kind = act.meta.get("kind").expect("No `kind` property in metadata.");

                match kind.as_str() {
                    "dsl" => {
                        let instr_file = block_on(registry::get_package_source(&name, &version, &kind))?;
                        if instr_file.exists() {
                            act.meta
                                .insert(String::from("instr_file"), String::from(instr_file.to_string_lossy()));
                        }
                    },
                    "cwl" | "ecu" | "oas" => {
                        let image_file = block_on(registry::get_package_source(&name, &version, &kind))?;
                        if image_file.exists() {
                            act.meta
                                .insert(String::from("image_file"), String::from(image_file.to_string_lossy()));
                        }
                    }
                    _ => {}
                }
            }
            Instruction::Sub(sub) => {
                if let Some(_) = sub.meta.get("kind") {
                    let name = sub.meta.get("name").expect("No `name` property in metadata.");
                    let version = sub.meta.get("version").expect("No `version` property in metadata.");

                    let package_dir = packages::get_package_dir(&name, Some(version))?;
                    let instructions_file = package_dir.join("instructions.yml");
                    let instructions_reader = BufReader::new(File::open(&instructions_file)?);

                    sub.instructions = serde_yaml::from_reader(instructions_reader)?;
                }

                debug!("Preprocess nested sub instrucations.");
                sub.instructions = preprocess_instructions(&sub.instructions)?;
            }
            _ => continue,
        }
    }

    debug!("{:#?}", instructions);

    Ok(instructions)
}

///
///
///
fn print(
    value: &Value,
    indent: bool,
) {
    if indent {
        print!("   ");
    }

    match value {
        Value::Array { entries, .. } => {
            println!("[");
            for entry in entries.iter() {
                print(entry, true);
            }
            println!("]")
        }
        Value::Boolean(b) => println!("{}", style(b).cyan()),
        Value::Integer(i) => println!("{}", style(i).cyan()),
        Value::Real(r) => println!("{}", style(r).cyan()),
        Value::Unicode(s) => println!("{}", style(s).cyan()),
        _ => println!("{:#?}", value),
    }
}
