// use escargot::format::{diagnostic::DiagnosticLevel, Message};
use cargo_metadata::{Message, diagnostic::DiagnosticLevel};
use std::{process::{Command, Stdio}, io::{BufRead, BufReader}};

mod diag;
mod print;
mod status;
mod unit_graph;

use status::Status;
use unit_graph::{Mode, UnitGraph};

fn main() -> anyhow::Result<()> {
    let output = Command::new("cargo")
        .args(&["build", "--message-format=json", "--unit-graph", "-Zunstable-options"])
        .output()?;

    let mut line = String::with_capacity(1024);
    (&output.stdout[..]).read_line(&mut line)?;
    let graph: UnitGraph = serde_json::from_str(&line)?;
    line.clear();

    let mut status = vec![Status::Unknown; graph.units.len()];

    println!();
    let mut tree_formatter = print::Formatter::new(&graph);
    tree_formatter.print(&status, false);

    let mut builder = Command::new("cargo");
    builder.args(&["build", "--message-format=json"]);

    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(|s| &**s) == Some("build-tree") {
        let _ = args.next();
    }

    for arg in args {
        builder.arg(arg);
    }

    let mut builder = builder.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

    let mut stdout = BufReader::new(builder.stdout.take().unwrap());

    while stdout.read_line(&mut line)? > 0 {
        match serde_json::from_str(&line) {
            Ok(Message::BuildScriptExecuted(msg)) => {
                let index = graph.units.iter().position(|unit| {
                    unit.mode == Mode::RunCustomBuild && unit.pkg_id == msg.package_id
                });
                if let Some(index) = index {
                    status[index] = Status::Done;
                }
                tree_formatter.print(&status, true);
            }
            Ok(Message::CompilerArtifact(msg)) => {
                let index = graph.units.iter().position(|unit| {
                    unit.mode == Mode::Build
                        && unit.target.kind == msg.target.kind
                        && unit.pkg_id == msg.package_id
                });
                if let Some(index) = index {
                    status[index] = Status::Done;
                }
                tree_formatter.print(&status, true);
            }
            Ok(Message::CompilerMessage(msg)) => {
                let index = graph.units.iter().position(|unit| {
                    unit.mode == Mode::Build
                        && unit.target.kind == msg.target.kind
                        && unit.pkg_id == msg.package_id
                });
                if let (Some(index), DiagnosticLevel::Error) = (index, msg.message.level) {
                    status[index] = Status::Error;
                }
                tree_formatter.clear();
                diag::emit(msg.message);
                tree_formatter.print(&status, false);
            }
            Ok(Message::BuildFinished(_)) => {
                break;
            }
            Ok(Message::TextLine(m)) => {
                tree_formatter.clear();
                dbg!(m);
                println!();
                tree_formatter.print(&status, false);
            }
            Ok(Message::Unknown) => {
                tree_formatter.clear();
                dbg!(&line);
                println!();
                tree_formatter.print(&status, false);
            }
            Err(e) => {
                tree_formatter.clear();
                dbg!(e);
                println!();
                tree_formatter.print(&status, false);
            }
        }
        line.clear();
    }

    Ok(())
}
