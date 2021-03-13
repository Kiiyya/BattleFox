// just a convenience vector creation macro, which converts all items to ascii.
macro_rules! veca {
    ($($x:expr),+ $(,)?) => {
        vec![
            $($x.into_ascii_string()?),+
        ]
    };
}

macro_rules! cmd_err {
    ($vis:vis $error_name:ident, $($error:ident),*) => {
        #[derive(Debug)]
        $vis enum $error_name {
            /// Some more low-level error returned by the rcon layer.
            /// For example TCP IO errors, connection closed, unknown RCON command,
            /// etc.
            Rcon(RconError),
            $($error),*
        }

        // impl <T> Result<T, $error_name> {
        //     fn unwrap_rcon(self) -> Result<T, RconError> {
        //         match self {
        //             Ok(val) => Ok(val),
        //             $error_name::Rcon(e) => Err(e),
        //             _ => panic!("Whoops, unwrapped non-rcon-error!"),
        //         }
        //     }
        // }

        impl From<RconError> for $error_name {
            fn from(e: RconError) -> Self {
                $error_name::Rcon(e)
            }
        }

        impl <O: Into<String> + IntoAsciiString> From<ascii::FromAsciiError<O>> for $error_name {
            fn from(e: ascii::FromAsciiError<O>) -> Self {
                $error_name::Rcon(RconError::NotAscii(e.into_source().into()))
            }
        }
    };
}
