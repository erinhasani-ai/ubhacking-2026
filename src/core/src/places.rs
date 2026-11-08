//! Named Places in the Room Twin (T31): fixed hover points in scan coordinates (metres, y up), read off the real
//! scan; user confirmation is tracked in T31.G3. The list is src/core/src/places.json, compiled in so the browser and the engine
//! read the same list. WebAssembly-safe: no file access.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Place {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

static PLACES: OnceLock<Vec<Place>> = OnceLock::new();

/// Parses a Place list and checks the names: non-empty, lowercase, trimmed, unique.
pub fn parse(json: &str) -> Result<Vec<Place>, String> {
    let places: Vec<Place> = serde_json::from_str(json).map_err(|e| format!("places list is malformed: {e}"))?;
    for (i, p) in places.iter().enumerate() {
        if p.name.is_empty() || p.name != p.name.trim().to_lowercase() {
            return Err(format!("place name {:?} must be non-empty, trimmed and lowercase", p.name));
        }
        if places[..i].iter().any(|q| q.name == p.name) {
            return Err(format!("place name {:?} appears twice", p.name));
        }
    }
    Ok(places)
}

/// Every known Place, parsed once from places.json.
pub fn all() -> &'static [Place] {
    PLACES.get_or_init(|| parse(include_str!("places.json")).expect("places.json is valid"))
}

/// Looks a Place up by name (exact after trim and lowercase). The error names every known Place.
pub fn find(name: &str) -> Result<&'static Place, String> {
    let wanted = name.trim().to_lowercase();
    all().iter().find(|p| p.name == wanted).ok_or_else(|| {
        let known: Vec<&str> = all().iter().map(|p| p.name.as_str()).collect();
        format!("unknown place {name:?}; known places: {}", known.join(", "))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_list_parses_with_lowercase_unique_names() {
        assert!(!all().is_empty());
        for p in all() {
            assert!(!p.name.is_empty() && p.name == p.name.to_lowercase() && p.name == p.name.trim(), "{}", p.name);
            assert_eq!(all().iter().filter(|q| q.name == p.name).count(), 1, "{} repeated", p.name);
        }
    }

    #[test]
    fn find_trims_and_lowercases() {
        let first = &all()[0];
        let messy = format!("  {}\t", first.name.to_uppercase());
        assert_eq!(find(&messy).unwrap(), first);
    }

    #[test]
    fn find_is_exact_not_prefix() {
        let first = &all()[0];
        assert!(find(&first.name[..first.name.len() - 1]).is_err());
    }

    #[test]
    fn unknown_name_error_lists_every_known_place() {
        let err = find("the moon").unwrap_err();
        for p in all() {
            assert!(err.contains(&p.name), "{err} lacks {}", p.name);
        }
        assert!(err.contains("the moon"));
    }

    #[test]
    fn parser_rejects_bad_lists() {
        assert!(parse("not json").is_err());
        assert!(parse(r#"[{"name":"door","x":1.0,"y":1.0}]"#).is_err(), "missing z");
        assert!(parse(r#"[{"name":"Door","x":0,"y":0,"z":0}]"#).is_err(), "uppercase");
        assert!(parse(r#"[{"name":"","x":0,"y":0,"z":0}]"#).is_err(), "empty");
        let dup = r#"[{"name":"a","x":0,"y":0,"z":0},{"name":"a","x":1,"y":0,"z":0}]"#;
        assert!(parse(dup).unwrap_err().contains("twice"));
        assert_eq!(parse(r#"[{"name":"a","x":0,"y":1,"z":2}]"#).unwrap().len(), 1);
    }
}
