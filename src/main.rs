use anyhow::anyhow;
use escargot::format::{diagnostic::DiagnosticLevel, Message};

mod diag;
mod print;
mod status;
mod unit_graph;

use status::Status;
use unit_graph::{Mode, UnitGraph};

fn main() -> anyhow::Result<()> {
    let mut msgs = escargot::CargoBuild::new()
        .arg("--unit-graph")
        .arg("-Z")
        .arg("unstable-options")
        .exec()?;

    let graph = msgs
        .next()
        .ok_or_else(|| anyhow!("no unit-graph"))??
        .decode_custom::<UnitGraph>()?;
    let mut status = vec![Status::Unknown; graph.units.len()];

    println!();

    let mut builder = escargot::CargoBuild::new();

    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(|s| &**s) == Some("build-tree") {
        let _ = args.next();
    }

    for arg in args {
        builder = builder.arg(arg);
    }

    let mut tree_formatter = print::Formatter::new(&graph);

    tree_formatter.print(&status, false);

    for msg in builder.exec()? {
        let msg = match msg {
            Ok(msg) => msg,
            Err(err) => return Err(err.into()),
        };
        if let Ok(msg) = msg.decode() {
            match msg {
                Message::BuildScriptExecuted(msg) => {
                    let index = graph.units.iter().position(|unit| {
                        unit.mode == Mode::RunCustomBuild && unit.pkg_id == msg.package_id
                    });
                    if let Some(index) = index {
                        status[index] = Status::Done;
                    }
                    tree_formatter.print(&status, true);
                }
                Message::CompilerArtifact(msg) => {
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
                Message::CompilerMessage(msg) => {
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
                Message::Unknown => {
                    tree_formatter.clear();
                    println!("{:?}", msg);
                    println!();
                    tree_formatter.print(&status, false);
                }
            }
        }
    }

    Ok(())
}
