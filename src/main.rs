// use escargot::format::{diagnostic::DiagnosticLevel, Message};
use cargo_metadata::{Message, diagnostic::DiagnosticLevel};
use tokio::{process::Command, io::{BufReader, AsyncBufReadExt as _}};
use std::{process::Stdio, iter::FromIterator};
use futures::stream::StreamExt as _;

mod diag;
mod print;
mod status;
mod unit_graph;

use status::Status;
use unit_graph::{Mode, UnitGraph};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(|s| &**s) == Some("build-tree") {
        let _ = args.next();
    }
    let args = Vec::from_iter(args);

    let output = Command::new("cargo")
        .args(&["build", "--message-format=json", "--unit-graph", "-Zunstable-options"])
        .args(&args)
        .output()
        .await?;

    let graph: UnitGraph = serde_json::from_slice(&output.stdout)?;

    let mut status = vec![Status::Unknown; graph.units.len()];

    println!();
    let mut tree_formatter = print::Formatter::new(&graph);
    tree_formatter.print(&status, false);

    let mut builder = Command::new("cargo")
        .args(&["build", "--message-format=json"]).args(&args)
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

    let stdout = BufReader::new(builder.stdout.take().unwrap());
    let stderr = BufReader::new(builder.stderr.take().unwrap());

    enum Item {
        Stdout(String),
        Stderr(String),
        Frame,
    }

    let mut items = futures::stream::select(futures::stream::select(
        stdout.lines().map(|l| l.map(Item::Stdout)),
        stderr.lines().map(|l| l.map(Item::Stderr))),
        tokio::time::interval(std::time::Duration::from_millis(100)).map(|_| Ok(Item::Frame)));

    while let Some(item) = items.next().await.transpose()? {
        match item {
            Item::Stdout(line) => {
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
                        // not possible to distinguish different platforms, just mark them all
                        // TODO: https://github.com/rust-lang/cargo/issues/12869
                        // && unit.platform == msg.platform
                        let indexes = graph.units.iter().zip(0..).filter_map(|(unit, position)| {
                            (unit.mode == Mode::Build
                                && unit.target.name == msg.target.name
                                && unit.target.kind == msg.target.kind
                                && unit.pkg_id == msg.package_id).then_some(position)
                        });
                        for index in indexes {
                            status[index] = Status::Done;
                        }
                        tree_formatter.print(&status, true);
                    }
                    Ok(Message::CompilerMessage(msg)) => {
                        // not possible to distinguish different platforms, just mark them all
                        // TODO: https://github.com/rust-lang/cargo/issues/12869
                        // && unit.platform == msg.platform
                        let indexes = graph.units.iter().zip(0..).filter_map(|(unit, position)| {
                            (unit.mode == Mode::Build
                                && unit.target.name == msg.target.name
                                && unit.target.kind == msg.target.kind
                                && unit.pkg_id == msg.package_id).then_some(position)
                        });
                        if let DiagnosticLevel::Error = msg.message.level {
                            for index in indexes {
                                status[index] = Status::Error;
                            }
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
            }
            Item::Stderr(line) => {
                let mut fragments = line.trim_start().split(' ');
                if fragments.next() == Some("Compiling") {
                    let name = fragments.next().unwrap_or_default();
                    let mut version = fragments.next().unwrap_or_default();
                    if version.starts_with('v') {
                        version = &version[1..];
                    }
                    // not possible to distinguish different platforms or targets, just mark them all
                    let indexes = graph.units.iter().zip(0..).filter_map(|(unit, position)| {
                        (unit.mode == Mode::Build
                            && unit.pkg_id.name == name
                            && unit.pkg_id.version == version).then_some(position)
                    });
                    for index in indexes {
                        status[index] = Status::Building;
                    }
                    tree_formatter.print(&status, true);
                }
            }
            Item::Frame => {
                tree_formatter.next_frame();
                tree_formatter.print(&status, true);
            }
        }
    }

    Ok(())
}
