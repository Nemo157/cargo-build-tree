use std::fmt;

const SPINNER: &[char] = &['◢', '◣', '◤', '◥'];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Status {
    Unknown,
    Building,
    Done,
    Error,
}

impl Status {
    pub fn display(self, frame: usize) -> impl fmt::Display {
        Framed(self, frame)
    }
}


struct Framed(Status, usize);

impl fmt::Display for Framed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Framed(Status::Unknown, _) => write!(f, "[36m…[0m")?,
            Framed(Status::Building, i) => write!(f, "[35m{}[0m", SPINNER[i % SPINNER.len()])?,
            Framed(Status::Done, _) => write!(f, "[32m✓[0m")?,
            Framed(Status::Error, _) => write!(f, "[31m[0m")?,
        }
        Ok(())
    }
}
