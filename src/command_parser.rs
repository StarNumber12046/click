use std::env::Args;

use crate::{
    errors::{
        CommandError,
        ParseError::{self, CommandNotFound},
    },
    installer::Installer,
};

pub trait CommandHandler {
    fn parse(&mut self, args: &mut Args) -> Result<(), ParseError>;
    fn execute(&self) -> Result<(), CommandError>;
}

pub fn handle_args(mut args: Args) -> Result<(), ParseError> {
    args.next(); // Remove initial binary argument

    let command = match args.next() {
        Some(c) => c,
        None => {
            // TODO(conaticus): Implement help menu
            println!("No help menu implemented yet.");
            return Ok(());
        }
    };

    let mut command_handler: Box<dyn CommandHandler> = match command.to_lowercase().as_str() {
        "install" => Box::new(Installer::default()),
        _ => return Err(CommandNotFound(command.to_string())),
    };

    command_handler.parse(&mut args)?;
    let command_result = command_handler.execute();

    if let Err(e) = command_result {
        println!("Command error: {e}");
    }

    Ok(())
}
