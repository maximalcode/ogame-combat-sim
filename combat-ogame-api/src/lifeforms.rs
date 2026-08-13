use std::collections::BTreeMap;

use combat_types::{LifeformTech, LifeformTechId, LifeformTechTable};

use crate::{Error, LifeformSettings, ServerData, TechnologyFactors};

/// Combat-stat lifeform researches loaded from one universe's `serverData.xml`.
///
/// This is the second source behind [`LifeformTechTable`]. The built-in table
/// remains untouched; callers choose whether they want its stable snapshot or
/// the per-universe configuration fetched by this crate.
#[derive(Debug, Clone, Default)]
pub struct ServerDataLifeformTechs {
    technologies: BTreeMap<LifeformTechId, LifeformTech>,
}

impl TryFrom<&ServerData> for ServerDataLifeformTechs {
    type Error = Error;

    fn try_from(server_data: &ServerData) -> Result<Self, Self::Error> {
        Self::try_from(&server_data.lifeform_settings)
    }
}

impl TryFrom<&LifeformSettings> for ServerDataLifeformTechs {
    type Error = Error;

    fn try_from(settings: &LifeformSettings) -> Result<Self, Self::Error> {
        let mut technologies = BTreeMap::new();

        for research in settings
            .lifeforms
            .iter()
            .flat_map(|lifeform| &lifeform.researches.researches)
            .filter(|research| research.effect_type == "BaseStatsBooster")
        {
            let mut targets = Vec::new();
            let mut per_level_percent = None;

            for factors in &research.factors.technologies {
                let Some(rate) = combat_rate(factors, research.technology_id)? else {
                    continue;
                };
                let target = factors.entity_id().ok_or_else(|| {
                    Error::InvalidLifeform(format!(
                        "research {} has combat factors without a numeric techId",
                        research.technology_id
                    ))
                })?;
                if let Some(expected) = per_level_percent {
                    if !approximately_equal(expected, rate) {
                        return Err(Error::InvalidLifeform(format!(
                            "research {} uses different rates for its targets",
                            research.technology_id
                        )));
                    }
                } else {
                    per_level_percent = Some(rate);
                }
                targets.push(target);
            }

            let Some(per_level_percent) = per_level_percent else {
                continue;
            };
            let technology = LifeformTech {
                id: research.technology_id,
                targets,
                per_level_percent: per_level_percent as f32,
            };
            if technologies
                .insert(research.technology_id, technology)
                .is_some()
            {
                return Err(Error::InvalidLifeform(format!(
                    "research {} occurs more than once",
                    research.technology_id
                )));
            }
        }

        Ok(Self { technologies })
    }
}

impl LifeformTechTable for ServerDataLifeformTechs {
    fn tech(&self, id: LifeformTechId) -> Option<LifeformTech> {
        self.technologies.get(&id).cloned()
    }

    fn ids(&self) -> Vec<LifeformTechId> {
        self.technologies.keys().copied().collect()
    }
}

fn combat_rate(
    factors: &TechnologyFactors,
    research_id: LifeformTechId,
) -> Result<Option<f64>, Error> {
    let rates = [
        factors.value("armorBase"),
        factors.value("shieldBase"),
        factors.value("weaponBase"),
    ];
    if rates.iter().all(Option::is_none) {
        return Ok(None);
    }
    let [Some(armour), Some(shield), Some(weapon)] = rates else {
        return Err(Error::InvalidLifeform(format!(
            "research {research_id} has only some combat-stat base factors"
        )));
    };
    if !approximately_equal(armour, shield) || !approximately_equal(armour, weapon) {
        return Err(Error::InvalidLifeform(format!(
            "research {research_id} uses different armour, shield and weapon rates"
        )));
    }
    Ok(Some(armour))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= f64::EPSILON * left.abs().max(right.abs()).max(1.0)
}
