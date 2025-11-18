#[macro_use]
extern crate async_trait;

mod cli;
mod config;
mod platform;
mod source;
mod util;

pub fn main() {
    cli::run();
}
