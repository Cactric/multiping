use console::{Term, style};
use std::io::Error;

fn say_hello() -> Result<(),Error> {
    let term = Term::stdout();
    term.clear_screen()?;
    term.write_line(&style("hello!").cyan().to_string())?;
    Ok(())
}

fn main() {
    let _ = say_hello();
    
    // Parse arguments
}
