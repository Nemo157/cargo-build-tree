use crate::status::Status;
use crate::unit_graph::{Mode, Unit, UnitGraph};
use fxhash::{FxBuildHasher, FxHashSet};
use std::fmt::Write;

pub struct Formatter<'a> {
    graph: &'a UnitGraph,
    frame: usize,
    lines: usize,
    buffer: String,
}

impl<'a> Formatter<'a> {
    pub fn new(graph: &'a UnitGraph) -> Self {
        Self {
            graph,
            frame: 0,
            lines: 0,
            buffer: String::new(),
        }
    }

    fn boring(
        &mut self,
        index: usize,
        unit: &Unit,
        status: &[Status],
        seen: &mut FxHashSet<usize>,
    ) -> Option<usize> {
        let mut total = 0;
        for dependency in &unit.dependencies {
            if seen.insert(dependency.index) {
                if let Some(count) = self.boring(
                    dependency.index,
                    &self.graph.units[dependency.index],
                    status,
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
        seen: &mut FxHashSet<usize>,
        indent: usize,
    ) -> usize {
        let mut total = 1;
        let extra = if unit.mode == Mode::RunCustomBuild {
            " (execute build script)"
        } else if unit.target.kind.contains(&"custom-build".to_owned()) {
            " (build build script)"
        } else {
            ""
        };
        write!(
            self.buffer,
            "{:1$} {2} {3}{4}",
            "", indent, &status[index].display(self.frame), unit.pkg_id, extra
        )
        .unwrap();
        if seen.insert(index) {
            let mut boring_seen = seen.clone();
            if let Some(count) = self.boring(index, unit, status, &mut boring_seen) {
                if count == 0 {
                    writeln!(self.buffer).unwrap();
                } else {
                    writeln!(self.buffer, " (+ {} other {})", count, &status[index].display(self.frame)).unwrap();
                }
            } else {
                writeln!(self.buffer).unwrap();
                let (done, others) = unit.dependencies.iter().partition::<Vec<_>, _>(|dep| status[dep.index] == Status::Done);
                for dependency in others {
                    total += self.println(
                        dependency.index,
                        &self.graph.units[dependency.index],
                        status,
                        seen,
                        indent + 2,
                    );
                }
                if !done.is_empty() {
                    write!(
                        self.buffer,
                        "{:1$} {2} ({3}",
                        "", indent + 2, Status::Done.display(self.frame), &self.graph.units[done[0].index].pkg_id
                    )
                    .unwrap();
                    for dep in done.iter().skip(1) {
                        write!(self.buffer, ", {}", &self.graph.units[dep.index].pkg_id).unwrap();
                    }
                    writeln!(self.buffer, ")").unwrap();
                    total += 1;
                }
            }
        } else {
            writeln!(self.buffer, " (*)").unwrap();
        }
        total
    }

    pub fn clear(&mut self) {
        print!("[{}F[J", self.lines + 1);
        self.lines = 0;
    }

    pub fn print(&mut self, status: &[Status], clear: bool) {
        self.buffer.clear();
        if clear {
            write!(self.buffer, "[{}F[J", self.lines + 1).unwrap();
        }
        let mut total = 2;
        let mut seen =
            FxHashSet::with_capacity_and_hasher(self.graph.units.len(), FxBuildHasher::default());
        for &root in &self.graph.roots {
            total += self.println(root, &self.graph.units[root], status, &mut seen, 0);
        }
        writeln!(self.buffer).unwrap();
        self.lines = total;
        print!("{}", self.buffer);
    }

    pub fn next_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }
}
