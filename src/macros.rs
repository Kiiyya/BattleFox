// just a convenience vector creation macro, which converts all items to ascii.
macro_rules! veca {
    ($($x:expr),+ $(,)?) => {
        vec![
            $($x.into_ascii_string()?),+
        ]
    };
}

macro_rules! cmd_err {
    ($vis:vis $error_name:ident, $($error:ident),+) => {
        #[derive(Debug)]
        $vis enum $error_name {
            /// Some more low-level error returned by the rcon layer.
            /// For example TCP IO errors, connection closed, unknown RCON command,
            /// etc.
            Rcon(RconError),
            $($error),+
        }

        impl From<RconError> for $error_name {
            fn from(e: RconError) -> Self {
                $error_name::Rcon(e)
            }
        }

        impl <T> From<ascii::FromAsciiError<T>> for $error_name {
            fn from(_e: ascii::FromAsciiError<T>) -> Self {
                $error_name::Rcon(RconError::NotAscii)
            }
        }
    };
}
