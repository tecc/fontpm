use std::{fmt, io};

pub struct ConsoleOutput {
    term: console::Term,
}
impl ConsoleOutput {
    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        io::Write::write_fmt(&mut &self.term, args)
    }
}
