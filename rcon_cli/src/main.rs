use std::{io::{BufRead, Write, stdout}, process::exit};

#[macro_use]
extern crate crossterm;

use ascii::IntoAsciiString;
use battlefield_rcon::rcon::{RconClient, RconConnectionInfo, RconError, RconQueryable, RconResult};
use clap::{Arg, SubCommand};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use dotenv::{dotenv, var};

fn get_rcon_coninfo() -> RconResult<RconConnectionInfo> {
    let ip = var("BFOX_RCON_IP").unwrap_or("127.0.0.1".into());
    let port = var("BFOX_RCON_PORT")
        .unwrap_or("47200".into())
        .parse::<u16>()
        .unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or("smurf".into());
    Ok(RconConnectionInfo {
        ip,
        port,
        password: password.into_ascii_string()?,
    })
}

#[allow(clippy::or_fun_call)]
#[tokio::main]
async fn main() -> RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.

    let coninfo = get_rcon_coninfo()?;

    let matches = clap::App::new("rcon-cli")
        .version("0.1")
        .about("Extremely simple and BF4-specifics-unaware (yet) library to send and receive strings. Uses the BFOX_RCON_IP, BFOX_RCON_PORT, and BFOX_RCON_PASSWORD environment variables for connection info. Alternatively, put a `.env` file in the working directory with those environment variables set.")
        .author("Kiiya (snoewflaek@gmail.com)")
        .arg(Arg::with_name("raw")
            .short("r")
            .long("--raw")
            .help("Prevents color output and ->, <-. Use this for automated scripts.")
        )
        .subcommand(SubCommand::with_name("query")
            .about("Send single query and print result, instead of going into interactive mode")
            .arg(Arg::with_name("query-words").min_values(1))
        )
        .get_matches();
    
    let raw = matches.is_present("raw");

    // println!("Connecting to {}:{} with password ***...", ip, port);
    let rcon = match RconClient::connect(&coninfo).await {
        Ok(rcon) => rcon,
        Err(err) => {
            println!("Failed to connect to Rcon at {}:{} with password ***: {:?}", coninfo.ip, coninfo.port, err);
            exit(-1);
        }
    };
    // let bf4 = Bf4Client::new(rcon).await.unwrap();
    // println!("Connected!");

    // if user provided "query" subcommand, just do that. Otherwise, go into interactive mode.
    if let Some(singlequery) = matches.subcommand_matches("query") {
        let words = singlequery.values_of("query-words").unwrap().collect::<Vec<_>>();
        handle_input_line(words, &rcon, raw).await?;
    } else {
        if !raw {
            print!("-> ");
            std::io::stdout().flush()?;
        }
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            let words = line.split(" ");
            handle_input_line(words, &rcon, raw).await?;
            if !raw {
                print!("-> ");
                std::io::stdout().flush()?;
            }
        }
    }

    Ok(())
}

async fn handle_input_line(words: impl IntoIterator<Item = &str>, rcon: &RconClient, raw: bool) -> RconResult<()> {
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
            if raw {
                println!("OK {}", str);
            } else {
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
        }
        Err(err) => {
            if !raw {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Black),
                    SetBackgroundColor(Color::Red),
                ).unwrap();
            }

            match err {
                RconError::Other(str) => {
                    // println!("{}", str.on_dark_red());
                    if raw {
                        println!("Error: {}", str);
                    } else {
                        execute!(
                            stdout(),
                            Print("<- Error".to_string()),
                            SetForegroundColor(Color::Red),
                            SetBackgroundColor(Color::Reset),
                            Print(" ".to_string()),
                            Print(str)
                        ).unwrap();
                    }
                },
                RconError::ConnectionClosed => {
                    print_error_type("Connection Closed", raw).unwrap();
                },
                RconError::InvalidArguments {our_query: _} => {
                    print_error_type("Invalid Arguments", raw).unwrap();
                },
                RconError::UnknownCommand {our_query: _} => {
                    print_error_type("Unknown Command", raw).unwrap();
                },
                _ => panic!("Unexpected error: {:?}", err),
            };
            if !raw {
                execute!(
                    stdout(),
                    ResetColor,
                    Print("\n".to_string())
                ).unwrap();
            }
        }
    }

    Ok(())
}

fn print_error_type(typ: &str, raw: bool) -> Result<(), crossterm::ErrorKind> {
    if raw {
        println!("{}", typ);
        Ok(())
    } else {
        execute!(
            stdout(),
            SetBackgroundColor(Color::DarkRed),
            Print("<- ".to_string()),
            Print(typ),
        )
    }
}
