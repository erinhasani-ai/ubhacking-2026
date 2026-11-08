//! The Command type: the one wire form every source (Gemini, Gesture, click, key) parses into.

use serde::{Deserialize, Serialize};

/// Drones in one Swarm; the limit for counts and drone ids.
pub const SWARM_SIZE: u8 = 6;

/// One instruction the Swarm can carry out. `Command::from_json` is the only validated entry point:
/// deriving `Deserialize` (e.g. `serde_json::from_str::<Command>`) skips validation and accepts arrays for objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Dispatch { drones: Drones, target: Target },
    Advance { drones: Drones },
    Recall { drones: Drones },
    // A struct variant, not a unit one: serde ignores unknown fields on unit variants of internally tagged enums.
    Abort {},
}

// Untagged, so each wire form is a bare string or a single-key object, as the contract writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Drones {
    Word(DroneWord),
    Count(CountBody),
    Ids(IdsBody),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DroneWord {
    All,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountBody {
    pub count: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdsBody {
    pub ids: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Word(TargetWord),
    Place(PlaceBody),
    Point(PointBody),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetWord {
    There,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceBody {
    pub place: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointBody {
    pub point: Point,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub n: f64,
    pub e: f64,
    pub d: f64,
}

impl Command {
    /// Parses and validates one Command. Every source must go through here.
    pub fn from_json(json: &str) -> Result<Command, String> {
        // The Value pre-pass only rejects shapes serde accepts but the contract forbids. The Command is built
        // from the original text, because a Value silently keeps the last of duplicate keys.
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        reject_arrays(&value, false)?;
        let command: Command = serde_json::from_str(json).map_err(|e| e.to_string())?;
        command.validate()?;
        Ok(command)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Command always serialises")
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Command::Dispatch { drones, target } => {
                drones.validate()?;
                target.validate()
            }
            Command::Advance { drones } | Command::Recall { drones } => drones.validate(),
            Command::Abort {} => Ok(()),
        }
    }
}

impl Drones {
    fn validate(&self) -> Result<(), String> {
        match self {
            Drones::Word(_) => Ok(()),
            Drones::Count(CountBody { count }) => in_swarm(*count),
            Drones::Ids(IdsBody { ids }) => {
                if ids.is_empty() {
                    return Err("drone ids are empty".into());
                }
                let mut seen = std::collections::HashSet::new();
                for &id in ids {
                    in_swarm(id)?;
                    if !seen.insert(id) {
                        return Err(format!("drone id {id} repeated"));
                    }
                }
                Ok(())
            }
        }
    }
}

impl Target {
    fn validate(&self) -> Result<(), String> {
        match self {
            Target::Word(_) => Ok(()),
            Target::Place(PlaceBody { place }) => {
                if place.trim().is_empty() || *place != place.trim().to_lowercase() {
                    return Err(format!("place {place:?} must be non-empty, trimmed and lowercase"));
                }
                Ok(())
            }
            Target::Point(PointBody { point: Point { n, e, d } }) => {
                if [n, e, d].iter().all(|v| v.is_finite()) {
                    Ok(())
                } else {
                    Err("point coordinates must be finite".into())
                }
            }
        }
    }
}

// serde's derived struct Deserialize also takes JSON arrays field by field; the only array on the wire is the value of "ids".
fn reject_arrays(value: &serde_json::Value, is_ids: bool) -> Result<(), String> {
    match value {
        serde_json::Value::Array(items) => {
            if !is_ids {
                return Err("arrays are only allowed as the value of ids".into());
            }
            items.iter().try_for_each(|v| reject_arrays(v, false))
        }
        serde_json::Value::Object(map) => map.iter().try_for_each(|(k, v)| reject_arrays(v, k == "ids")),
        // No wire form uses null; serde would take {"all":null} as the bare word "all".
        serde_json::Value::Null => Err("null is not allowed".into()),
        _ => Ok(()),
    }
}

fn in_swarm(id: u8) -> Result<(), String> {
    if (1..=SWARM_SIZE).contains(&id) {
        Ok(())
    } else {
        Err(format!("drone {id} outside 1-{SWARM_SIZE}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn round_trips(wire: Value) {
        let text = wire.to_string();
        let command = Command::from_json(&text).unwrap_or_else(|e| panic!("{text} rejected: {e}"));
        let back: Value = serde_json::from_str(&command.to_json()).unwrap();
        assert_eq!(back, wire);
    }

    fn rejected(wire: Value) {
        let text = wire.to_string();
        assert!(Command::from_json(&text).is_err(), "accepted: {text}");
    }

    #[test]
    fn each_kind_parses_and_round_trips() {
        round_trips(json!({"kind": "dispatch", "drones": {"count": 2}, "target": {"place": "door"}}));
        round_trips(json!({"kind": "dispatch", "drones": {"ids": [1, 3]}, "target": "there"}));
        round_trips(json!({"kind": "dispatch", "drones": "all", "target": {"point": {"n": 1.5, "e": -2.0, "d": 0.0}}}));
        round_trips(json!({"kind": "advance", "drones": "all"}));
        round_trips(json!({"kind": "recall", "drones": {"ids": [6]}}));
        round_trips(json!({"kind": "abort"}));
    }

    #[test]
    fn arrays_are_rejected_except_as_ids() {
        rejected(json!({"kind": "advance", "drones": [2]}));
        rejected(json!({"kind": "advance", "drones": [[1, 3]]}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": ["door"]}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": [[1, 2, 3]]}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"point": [1, 2, 3]}}));
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        for text in [
            r#"{"kind":"abort","kind":"dispatch","drones":"all","target":"there"}"#,
            r#"{"kind":"advance","kind":"abort"}"#,
            r#"{"kind":"advance","drones":{"count":9,"count":2}}"#,
            r#"{"kind":"dispatch","drones":"all","target":"there","target":{"place":"door"}}"#,
            r#"{"kind":"advance","drones":{"ids":[1],"ids":[2]}}"#,
            r#"{"kind":"dispatch","drones":"all","target":{"place":"a","place":"door"}}"#,
            r#"{"kind":"dispatch","drones":"all","target":{"point":{"n":1,"n":2,"e":0,"d":0}}}"#,
        ] {
            assert!(Command::from_json(text).is_err(), "accepted: {text}");
        }
    }

    #[test]
    fn map_form_of_a_word_is_rejected() {
        rejected(json!({"kind": "advance", "drones": {"all": null}}));
        rejected(json!({"kind": "advance", "drones": {"all": {}}}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"there": null}}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"there": {}}}));
    }

    #[test]
    fn unknown_fields_are_rejected_at_every_level() {
        rejected(json!({"kind": "advance", "drones": "all", "x": 1}));
        rejected(json!({"kind": "advance", "drones": {"count": 2, "x": 1}}));
        rejected(json!({"kind": "advance", "drones": {"ids": [1], "x": 1}}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"place": "door", "x": 1}}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"point": {"n": 1, "e": 2, "d": 3, "x": 4}}}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"point": {"n": 1, "e": 2, "d": 3}, "x": 4}}));
        rejected(json!({"kind": "abort", "x": 1}));
    }

    #[test]
    fn unknown_kind_is_rejected() {
        rejected(json!({"kind": "land", "drones": "all"}));
    }

    #[test]
    fn drone_counts_outside_the_swarm_are_rejected() {
        rejected(json!({"kind": "advance", "drones": {"count": 0}}));
        rejected(json!({"kind": "advance", "drones": {"count": 7}}));
    }

    #[test]
    fn bad_id_lists_are_rejected() {
        rejected(json!({"kind": "recall", "drones": {"ids": []}}));
        rejected(json!({"kind": "recall", "drones": {"ids": [2, 2]}}));
        rejected(json!({"kind": "recall", "drones": {"ids": [0]}}));
        rejected(json!({"kind": "recall", "drones": {"ids": [7]}}));
    }

    #[test]
    fn empty_or_unnormalised_place_is_rejected() {
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"place": "  "}}));
        rejected(json!({"kind": "dispatch", "drones": "all", "target": {"place": "Door"}}));
    }

    #[test]
    fn dispatch_without_target_is_rejected() {
        rejected(json!({"kind": "dispatch", "drones": "all"}));
    }
}
