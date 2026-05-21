use clap::{ArgMatches, Command};
use dali_tools::drivers::driver::DaliDriver;
use std::error::Error;
use std::pin::Pin;

pub struct ToolContext {
    pub driver: Box<dyn DaliDriver>,
}

pub type ExecuteTool = for<'a> fn(
    &'a mut ToolContext,
    &'a ArgMatches,
)
    -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error>>> + 'a>>;
pub struct SubTool {
    pub sub_command: Command,
    pub execute: ExecuteTool,
}
