pub mod command;
pub mod room_twin;
pub mod vehicle;

/// Crate version from Cargo.toml, reported by the WebAssembly build.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_three_dot_separated_integers() {
        let parts: Vec<&str> = version().split('.').collect();
        assert_eq!(parts.len(), 3, "version was {}", version());
        for part in parts {
            assert!(part.parse::<u32>().is_ok(), "not an integer: {part}");
        }
    }
}
