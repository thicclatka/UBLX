#[macro_export]
macro_rules! mod_pub {
    [ $( $name:ident $(,)? )+ ] => {
        $(
            pub mod $name;
        )+
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
