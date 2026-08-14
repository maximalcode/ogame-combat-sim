mod combat;
pub mod economics;
mod entity;
mod instant;
pub mod report_builder;
mod scaling;
mod simulator;
mod stats;

pub use combat::Combat;
pub use report_builder::ReportBuilder;
pub use scaling::{
    calculate_downscale_factor, downscale_party, should_downscale, upscale_result,
    upscale_result_with_originals,
};
pub use simulator::Simulator;

#[cfg(test)]
mod tests;
