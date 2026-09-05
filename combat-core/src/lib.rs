mod combat;
pub mod economics;
mod entity;
mod instant;
pub mod report_builder;
mod scaling;
mod simulator;
pub mod stats;

pub use combat::Combat;
pub use report_builder::ReportBuilder;
pub use scaling::{
    calculate_downscale_factor, downscale_party, should_downscale, upscale_result,
    upscale_result_with_originals,
};
pub use simulator::Simulator;
pub use stats::{ModifiedStats, StatsCache};

#[cfg(test)]
mod tests;
