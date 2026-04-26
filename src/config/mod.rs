//! TOML config, ublx paths, CLI/options validation, theme/toast/parallelism knobs, and streaming
//! / profile wiring. Re-exports the stable config surface for the app.

mod opts;
mod parallel;
mod paths;
mod profile;
mod streaming;
mod theme;
mod toast;
mod validation;

pub use opts::*;
pub use parallel::*;
pub use paths::*;
pub use profile::*;
pub use streaming::*;
pub use toast::*;
pub use validation::*;
