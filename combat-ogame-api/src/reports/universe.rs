//! Resolve a report's universe from the existing public `serverData.xml`
//! client.

use super::{Candidate, EvidenceSource, PinnedUniverse, PinnedUniverseSettings};
use crate::{Error, OGameClient, ServerData};
use std::fmt;

/// A failure while turning one public metadata response into a pinned
/// universe snapshot.
#[derive(Debug)]
pub enum UniverseResolutionError {
    /// The existing public metadata client failed. Its cache and rate limit
    /// behavior remain the transport boundary for this operation.
    Metadata(Error),
    /// The response was for another community or universe.
    WrongIdentity {
        expected_community: String,
        expected_universe: u32,
        metadata_community: String,
        metadata_universe: u32,
    },
    /// A public value could not be represented by the checked completion
    /// schema without rounding or activating an implicit default.
    InvalidSetting { field: &'static str, value: String },
}

impl fmt::Display for UniverseResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metadata(error) => write!(formatter, "fetch public universe metadata: {error}"),
            Self::WrongIdentity {
                expected_community,
                expected_universe,
                metadata_community,
                metadata_universe,
            } => write!(
                formatter,
                "public universe metadata identity {metadata_community}-{metadata_universe} does not match report {expected_community}-{expected_universe}"
            ),
            Self::InvalidSetting { field, value } => {
                write!(
                    formatter,
                    "public universe setting {field} has unsupported value {value}"
                )
            }
        }
    }
}

impl std::error::Error for UniverseResolutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Metadata(error) => Some(error),
            Self::WrongIdentity { .. } | Self::InvalidSetting { .. } => None,
        }
    }
}

/// Resolve the report's current universe snapshot through the existing
/// public `OGame` XML client. The client supplies the established disk cache and
/// per-host rate limit; report IDs and report payloads never enter this path.
pub async fn resolve_current_universe(
    candidate: &Candidate,
    client: &OGameClient,
) -> Result<PinnedUniverse, UniverseResolutionError> {
    let metadata = client
        .server_data()
        .await
        .map_err(UniverseResolutionError::Metadata)?;
    pinned_universe_from_server_data(
        &candidate.provenance.community,
        candidate.provenance.universe,
        &metadata,
    )
}

/// Convert one already fetched public `serverData.xml` response into a
/// complete current snapshot. This pure seam keeps offline tests independent
/// of network access while the async resolver above retains the normal client
/// cache and rate-limit behavior.
pub fn pinned_universe_from_server_data(
    community: &str,
    universe: u32,
    metadata: &ServerData,
) -> Result<PinnedUniverse, UniverseResolutionError> {
    let metadata_community = metadata.language.clone();
    let metadata_universe = metadata.number;
    if metadata_community != community || metadata_universe != universe {
        return Err(UniverseResolutionError::WrongIdentity {
            expected_community: community.to_owned(),
            expected_universe: universe,
            metadata_community,
            metadata_universe,
        });
    }

    let settings = PinnedUniverseSettings {
        galaxies: Some(checked_u8("galaxies", metadata.galaxies.into(), 1, 9)?),
        systems: Some(checked_u16("systems", metadata.systems.into(), 1, 499)?),
        donut_galaxy: Some(metadata.donut_galaxy),
        donut_systems: Some(metadata.donut_system),
        fleet_speed: Some(checked_u8(
            "fleet_speed",
            metadata.speed_fleet_peaceful,
            1,
            u32::from(u8::MAX),
        )?),
        debris_fleet: Some(checked_factor("debris_factor", metadata.debris_factor)?),
        debris_defence: Some(checked_factor(
            "debris_factor_def",
            metadata.debris_factor_def,
        )?),
        debris_deuterium: Some(metadata.deuterium_in_debris.ok_or_else(|| {
            UniverseResolutionError::InvalidSetting {
                field: "deuterium_in_debris",
                value: "missing".to_owned(),
            }
        })?),
        deuterium_save_factor: Some(checked_factor(
            "global_deuterium_save_factor",
            metadata.global_deuterium_save_factor,
        )?),
    };

    if metadata.version.trim().is_empty() {
        return Err(UniverseResolutionError::InvalidSetting {
            field: "version",
            value: metadata.version.clone(),
        });
    }

    Ok(PinnedUniverse {
        community: community.to_owned(),
        universe,
        settings,
        source: EvidenceSource::PublicMetadata,
        source_timestamp: Some(metadata.timestamp),
        source_version: Some(metadata.version.clone()),
        current: Some(true),
        // Current metadata must be acknowledged at the completion boundary;
        // this value is deliberately not inferred from the report timestamp.
        acknowledged_current: Some(false),
    })
}

fn checked_u8(
    field: &'static str,
    value: u32,
    minimum: u32,
    maximum: u32,
) -> Result<u8, UniverseResolutionError> {
    if !(minimum..=maximum).contains(&value) {
        return Err(UniverseResolutionError::InvalidSetting {
            field,
            value: value.to_string(),
        });
    }
    u8::try_from(value).map_err(|_| UniverseResolutionError::InvalidSetting {
        field,
        value: value.to_string(),
    })
}

fn checked_u16(
    field: &'static str,
    value: u32,
    minimum: u32,
    maximum: u32,
) -> Result<u16, UniverseResolutionError> {
    if !(minimum..=maximum).contains(&value) {
        return Err(UniverseResolutionError::InvalidSetting {
            field,
            value: value.to_string(),
        });
    }
    u16::try_from(value).map_err(|_| UniverseResolutionError::InvalidSetting {
        field,
        value: value.to_string(),
    })
}

fn checked_factor(field: &'static str, value: f64) -> Result<u8, UniverseResolutionError> {
    let percentage = value * 100.0;
    if !percentage.is_finite()
        || !(0.0..=100.0).contains(&percentage)
        || (percentage.round() - percentage).abs() > 1e-9
    {
        return Err(UniverseResolutionError::InvalidSetting {
            field,
            value: value.to_string(),
        });
    }
    Ok(percentage.round() as u8)
}
