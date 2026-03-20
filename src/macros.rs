#[macro_export]
macro_rules! mod_pub {
    [ $( $name:ident $(,)? )+ ] => {
        $(
            pub mod $name;
        )+
    };
}

/// If `$result` is `Err(e)`, log with `log::error!($($msg)*, e)` and exit with [`crate::utils::EXIT_ERROR`]. Otherwise unwrap the `Ok` value.
#[macro_export]
macro_rules! fatal {
    ($result:expr, $($msg:tt)*) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                log::error!($($msg)*, e);
                $crate::utils::exit_error();
            }
        }
    };
}

#[macro_export]
macro_rules! mod_priv {
    [ $( $name:ident $(,)? )+ ] => {
        $(
            mod $name;
        )+
    };
}
