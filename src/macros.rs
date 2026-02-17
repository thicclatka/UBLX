#[macro_export]
macro_rules! mod_pub {
    [ $( $name:ident $(,)? )+ ] => {
        $(
            pub mod $name;
        )+
    };
}

/// If `$result` is `Err(e)`, log with `log::error!($($msg)*, e)` and exit(1). Otherwise unwrap the `Ok` value.
#[macro_export]
macro_rules! fatal {
    ($result:expr, $($msg:tt)*) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                log::error!($($msg)*, e);
                std::process::exit(1);
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
