#[cfg(feature = "debug")]
pub use backtrace;
use crate::error::Error;

#[derive(PartialEq, Eq)]
pub enum OutputKind {
    Ok,
    Error,
    Warning,
    Info,
    Debug,
    Trace
}
pub struct OutputRecord {
    pub kind: OutputKind,
    #[cfg(feature = "debug")]
    pub module: String,
    #[cfg(feature = "debug")]
    pub line: Option<u32>,
    pub message: String
}

pub trait CliOutput: Sync {
    fn is_enabled(&self, kind: OutputKind) -> bool;
    fn log(&self, record: OutputRecord) -> Result<(), Error>;
}

static mut IMPL: Option<&dyn CliOutput> = None;

const MISSING_IMPL_MESSAGE: &str = "Log implementation is not set";

pub fn get_impl() -> &'static dyn CliOutput {
    unsafe {
        return IMPL.expect(MISSING_IMPL_MESSAGE);
    }
}

pub fn set_impl(reference: &'static dyn CliOutput) {
    unsafe {
        if let Some(_) = IMPL {
            panic!("Cannot set log implementation twice"); // Can't be bothered with proper errors
        }
        IMPL = Some(reference);
    }
}

macro_rules! declare_level_macro {
    (($d:tt), $name:ident, $level:ident) => {
        #[macro_export]
        macro_rules! $name {
            ($fmt:literal $d(, $fmt_arg: expr)*) => {
                #[cfg(feature = "debug")]
                let mut backtrace = $crate::output::backtrace::Backtrace::new();
                let i = $crate::output::get_impl();

                if i.is_enabled($crate::output::OutputKind::$level) {
                    let message = format!($fmt $d(, $fmt_arg)*);
                    #[cfg(feature = "debug")]
                    let frames = backtrace.frames();
                    #[cfg(feature = "debug")]
                    let symbols = frames[0].symbols();
                    #[cfg(feature = "debug")]
                    let symbol = &symbols[0];
                    if let Err(e) = i.log($crate::output::OutputRecord {
                        kind: $crate::output::OutputKind::$level,
                        #[cfg(feature = "debug")]
                        module: String::from(std::module_path!()),
                        #[cfg(feature = "debug")]
                        line: symbol.lineno(),
                        message
                    }) {
                        panic!("Could not print: {}", e);
                    }
                }
                ()
            }
        }
    };
    ($name:ident, $level:ident) => {
        declare_level_macro!(($), $name, $level);
    };
}

declare_level_macro!(error, Error);
declare_level_macro!(warning, Warning);
declare_level_macro!(info, Info);
declare_level_macro!(ok, Ok);
declare_level_macro!(debug, Debug);
declare_level_macro!(trace, Trace);
