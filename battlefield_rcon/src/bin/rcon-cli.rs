use std::io::{BufRead, Write, stdout};

#[macro_use]
extern crate crossterm;

use ascii::IntoAsciiString;
use battlefield_rcon::rcon::{RconClient, RconError, RconQueryable, RconResult};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use dotenv::{dotenv, var};

#[allow(clippy::or_fun_call)]
#[tokio::main]
async fn main() -> RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.

    let ip = var("BFOX_RCON_IP").unwrap_or("127.0.0.1".into());
    let port = var("BFOX_RCON_PORT")
        .unwrap_or("47200".into())
        .parse::<u16>()
        .unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or("smurf".into());

    println!("Connecting to {}:{} with password ***...", ip, port);
    let rcon = RconClient::connect((ip.as_str(), port, password.as_str())).await?;
    // let bf4 = Bf4Client::new(rcon).await.unwrap();
    println!("Connected!");

    print!("-> ");
    std::io::stdout().flush()?;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let words = line.split(" ");
        let mut words_ascii = Vec::new();
        for word in words {
            words_ascii.push(word.into_ascii_string()?);
        }

        let result = rcon.query(&words_ascii,
            |ok| Ok(ok.to_owned()),
            |err| Some(RconError::other(err)),
        ).await;

        match result {
            Ok(ok) => {
                let mut str = String::new();
                for word in ok {
                    str.push(' ');
                    str.push_str(word.as_str());
                }
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Black),
                    SetBackgroundColor(Color::Green),
                    Print("<- OK".to_string()),
                    SetForegroundColor(Color::Green),
                    SetBackgroundColor(Color::Reset),
                    Print(str),
                    ResetColor,
                    Print("\n".to_string())
                ).unwrap();
            }
            Err(err) => {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Black),
                    SetBackgroundColor(Color::Red),
                ).unwrap();

                match err {
                    RconError::Other(str) => {
                        // println!("{}", str.on_dark_red());
                        execute!(
                            stdout(),
                            Print("<- Error".to_string()),
                            SetForegroundColor(Color::Red),
                            SetBackgroundColor(Color::Reset),
                            Print(" ".to_string()),
                            Print(str)
                        ).unwrap();
                    },
                    RconError::ConnectionClosed => {
                        print_error_type("Connection Closed").unwrap();
                    },
                    RconError::InvalidArguments {our_query: _} => {
                        print_error_type("Invalid Arguments").unwrap();
                    },
                    RconError::UnknownCommand {our_query: _} => {
                        print_error_type("Unknown Command").unwrap();
                    },
                    _ => panic!("Unexpected error: {:?}", err),
                };
                execute!(
                    stdout(),
                    ResetColor,
                    Print("\n".to_string())
                ).unwrap();
            }
        }

        print!("-> ");
        std::io::stdout().flush()?;
    }

    Ok(())
}

fn print_error_type(typ: &str) -> Result<(), crossterm::ErrorKind> {
    execute!(
        stdout(),
        SetBackgroundColor(Color::DarkRed),
        Print("<- ".to_string()),
        Print(typ),
    )
}
