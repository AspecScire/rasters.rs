pub use clap::{App, Arg};
pub use inflector::Inflector;

#[macro_export]
macro_rules! args_parser {
    ($name:expr) => {{
        $crate::cli::args::App::new($name)
            .version(clap::crate_version!())
            .author(clap::crate_authors!())
    }};
}

#[macro_export]
macro_rules! arg {
    ($name:expr) => {{
        use inflector::Inflector;
        $crate::cli::args::Arg::with_name($name).value_name(&$name.to_screaming_snake_case())
    }};
}

#[macro_export]
macro_rules! opt {
    ($name:expr) => {{
        use inflector::Inflector;
        $crate::cli::args::Arg::with_name($name)
            .long(&$name.to_kebab_case())
            .value_name(&$name.to_screaming_snake_case())
    }};
}
