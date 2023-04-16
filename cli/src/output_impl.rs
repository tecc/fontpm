use std::io::Write;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use fontpm_api::error::Error;
use fontpm_api::output::{CliOutput, OutputKind, OutputRecord, set_impl};

#[derive(PartialEq, Eq)]
pub enum OutputLevel {
    VerySilent,
    Silent,
    Normal,
    Verbose,
    VeryVerbose
}

struct OutputImpl {
    output_level: OutputLevel,
    stdout: Box<BufferWriter>,
    stderr: Box<BufferWriter>
}
impl OutputImpl {
    fn new(level: OutputLevel) -> Self {
        return OutputImpl {
            output_level: level,
            stdout: Box::new(BufferWriter::stdout(ColorChoice::Auto)),
            stderr: Box::new(BufferWriter::stderr(ColorChoice::Auto))
        }
    }
}

const SPACE: &[u8] = &[' ' as u8];
const NEWLINE: &[u8] = &['\n' as u8];

impl CliOutput for OutputImpl {
    fn is_enabled(&self, kind: OutputKind) -> bool {
        use OutputKind::*;
        use OutputLevel::*;
        match (&self.output_level, kind) {
            (_, Error) => true,
            (VeryVerbose, _) => true,
            (Verbose, Warning | Info | Ok | Debug) => true,
            (Normal, Warning | Info | Ok) => true,
            (Silent, Warning | Ok) => true,
            (VerySilent, _) => false,
            _ => false
        }
    }

    fn log(&self, record: OutputRecord) -> Result<(), Error> {
        let message = {
            let mut message = termcolor::Buffer::ansi();
            let mut prefix_col = ColorSpec::new();
            let (color, dimmed, v) = match record.kind {
                OutputKind::Ok => (Color::Green, false, "ok!"),
                OutputKind::Error => (Color::Red, false, "error!"),
                OutputKind::Warning => (Color::Yellow, false, "warning!"),
                OutputKind::Info => (Color::Blue, false, "info"),
                OutputKind::Debug => (Color::Cyan, true, "debug"),
                OutputKind::Trace => (Color::White, true, "trace")
            };
            prefix_col.set_fg(Some(color)).set_bold(true).set_dimmed(dimmed);
            message.set_color(&prefix_col)?;
            message.write_fmt(format_args!("{}", v))?;

            message.reset()?;
            message.write(SPACE)?;

            message.write(record.message.as_bytes())?;
            if let Some(msg) = record.message.chars().last() {
                if msg != '\n' {
                    message.write(NEWLINE)?;
                }
            }
            message
        };

        let writer = match record.kind {
            OutputKind::Ok | OutputKind::Info | OutputKind::Debug | OutputKind::Trace => &self.stdout,
            OutputKind::Error | OutputKind::Warning => &self.stderr
        };
        let writer = writer;
        // let line = format!("{} {}", prefix, record.message);
        writer.print(&message)?;
        Ok(())
    }
}

static mut INSTANCE: Option<OutputImpl> = None;

pub fn init(level: OutputLevel) {
    unsafe {
        INSTANCE = Some(OutputImpl::new(level));
        set_impl(INSTANCE.as_ref().unwrap());
    }
}