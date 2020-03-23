use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Status {
    Unknown,
    Building,
    Done,
    Error,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Unknown => write!(f, "[36m…[0m")?,
            Status::Building => write!(f, "[35m↻[0m")?,
            Status::Done => write!(f, "[32m✓[0m")?,
            Status::Error => write!(f, "[31m[0m")?,
        }
        Ok(())
    }
}
