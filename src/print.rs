use crate::status::Status;
use crate::unit_graph::{Unit, UnitGraph};
use fxhash::{FxBuildHasher, FxHashSet};
use std::fmt::Write;
use cargo_metadata::diagnostic::Diagnostic;

pub struct Formatter<'a> {
    graph: &'a UnitGraph,
    frame: usize,
    buffer: String,
}

impl<'a> Formatter<'a> {
    pub fn new(graph: &'a UnitGraph) -> Self {
        Self {
            graph,
            frame: 0,
            buffer: String::new(),
        }
    }

    fn boring(
        &mut self,
        index: usize,
        unit: &Unit,
        status: &[Status],
        diagnostics: &[Vec<Diagnostic>],
        seen: &mut FxHashSet<usize>,
    ) -> Option<usize> {
        let mut total = 0;
        if !diagnostics[index].is_empty() {
            return None;
        }
        for dependency in &unit.dependencies {
            if seen.insert(dependency.index) {
                if let Some(count) = self.boring(
                    dependency.index,
                    &self.graph.units[dependency.index],
                    status,
                    diagnostics,
                    seen,
                ) {
                    total += count + 1;
                } else {
                    return None;
                }
                if status[dependency.index] != status[index] {
                    return None;
                }
            }
        }
        Some(total)
    }

    fn println(
        &mut self,
        index: usize,
        unit: &Unit,
        status: &[Status],
        diagnostics: &[Vec<Diagnostic>],
        seen: &mut FxHashSet<usize>,
        indent: usize,
        platform: Option<&str>,
    ) {
        write!(
            self.buffer,
            "{:1$} {2} {unit}",
            "", indent, &status[index].display(self.frame)
        )
        .unwrap();

        if unit.platform.as_deref() != platform {
            if let Some(unit_platform) = &unit.platform {
                write!(self.buffer, " ({unit_platform})").unwrap();
            } else {
                self.buffer.push_str(" (host)");
            }
        }

        if seen.insert(index) {
            let mut boring_seen = seen.clone();
            if let Some(count) = self.boring(index, unit, status, diagnostics, &mut boring_seen) {
                if count == 0 {
                    writeln!(self.buffer).unwrap();
                } else {
                    writeln!(self.buffer, " (+ {} other {})", count, &status[index].display(self.frame)).unwrap();
                }
            } else {
                writeln!(self.buffer).unwrap();
                for diagnostic in &diagnostics[index] {
                    writeln!(self.buffer, "{:1$} ⚠️ {2:?}: {3}", "", indent + 2, diagnostic.level, diagnostic.message);
                }
                let (done, others) = unit.dependencies.iter().partition::<Vec<_>, _>(|dep| status[dep.index] == Status::Done);
                for dependency in others {
                    self.println(
                        dependency.index,
                        &self.graph.units[dependency.index],
                        status,
                        diagnostics,
                        seen,
                        indent + 2,
                        platform,
                    );
                }
                if !done.is_empty() {
                    write!(
                        self.buffer,
                        "{:1$} {2} ({3}",
                        "", indent + 2, Status::Done.display(self.frame), &self.graph.units[done[0].index].target.name
                    )
                    .unwrap();
                    for dep in done.iter().skip(1) {
                        write!(self.buffer, ", {}", &self.graph.units[dep.index].target.name).unwrap();
                    }
                    writeln!(self.buffer, ")").unwrap();
                }
            }
        } else {
            writeln!(self.buffer, " (*)").unwrap();
        }
    }

    pub fn print(&mut self, status: &[Status], diagnostics: &[Vec<Diagnostic>]) {
        self.buffer.clear();
        write!(self.buffer, "[?1049h").unwrap();
        let mut seen =
            FxHashSet::with_capacity_and_hasher(self.graph.units.len(), FxBuildHasher::default());
        let platform = self.graph.units[self.graph.roots[0]].platform.as_deref();
        for &root in &self.graph.roots {
            self.println(root, &self.graph.units[root], status, diagnostics, &mut seen, 0, platform);
        }
        writeln!(self.buffer).unwrap();
        write!(self.buffer, "[?1049l").unwrap();
        print!("{}", self.buffer);
    }

    pub fn next_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }
}
