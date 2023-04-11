use std::fmt;

/// Format the given part and whole as a percentage.
pub fn percentage(part: u32, total: u32) -> impl fmt::Display {
    Percentage(part, total)
}

#[derive(Clone, Copy)]
struct Percentage(u32, u32);

impl fmt::Display for Percentage {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Percentage(part, total) = *self;

        let total = match total {
            0 => return write!(fmt, "0%"),
            total => total,
        };

        let p = (part * 10_000) / total;
        write!(fmt, "{}", p / 100)?;

        match p % 100 {
            0 => (),
            n => write!(fmt, ".{}", n)?,
        };

        fmt.write_str("%")
    }
}
