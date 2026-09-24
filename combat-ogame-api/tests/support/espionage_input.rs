use combat_ogame_api::reports::{CompletionInput, ReportId, parse_report};
use serde_json::json;

#[path = "comparison_input.rs"]
#[allow(dead_code)]
mod support;

pub fn scenario() -> CompletionInput {
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"event_timestamp":1_700_000_000,"failed_ships":false,"failed_defense":false,"failed_research":false},
        "details":{"ships":[{"ship_type":204,"count":12}],"defense":[],
        "research":[{"research_type":109,"level":10},{"research_type":110,"level":10},{"research_type":111,"level":10}]}
    }});
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    let mut evidence = support::evidence();
    evidence.participants.get_mut("A1").unwrap().entities = Some([(204, 20)].into());
    evidence.participants.get_mut("D1").unwrap().technology = None;
    CompletionInput {
        candidate: parse_report(&id, &payload.to_string()).unwrap(),
        evidence,
        universe: support::universe(),
    }
}
