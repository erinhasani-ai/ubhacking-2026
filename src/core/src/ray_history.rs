//! Pointing-ray history for Put-That-There: remembers recent rays on the engine clock
//! so the word "there" can be matched to where the user pointed when it was spoken.

use std::collections::VecDeque;

/// A pointing ray in the Mission frame; `dir` is a unit vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: [f64; 3],
    pub dir: [f64; 3],
}

/// A spoken word with its start time already converted to the engine clock.
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub start_ms: u64,
}

/// Rays from the last `window_ms` of engine time.
pub struct RayHistory {
    window_ms: u64,
    samples: VecDeque<(u64, Ray)>,
}

impl RayHistory {
    pub fn new(window_ms: u64) -> Self {
        Self { window_ms, samples: VecDeque::new() }
    }

    /// Adds a sample; one older than the newest is ignored, and samples older than newest - window are dropped.
    pub fn push(&mut self, t_ms: u64, ray: Ray) {
        if self.samples.back().is_some_and(|&(newest, _)| t_ms < newest) {
            return;
        }
        self.samples.push_back((t_ms, ray));
        let cutoff = t_ms.saturating_sub(self.window_ms);
        while self.samples.front().is_some_and(|&(t, _)| t < cutoff) {
            self.samples.pop_front();
        }
    }

    /// The sample nearest `t_ms`, if it is within `max_gap_ms` (no interpolation).
    pub fn at(&self, t_ms: u64, max_gap_ms: u64) -> Option<(u64, Ray)> {
        self.samples
            .iter()
            .min_by_key(|(t, _)| t.abs_diff(t_ms))
            .filter(|(t, _)| t.abs_diff(t_ms) <= max_gap_ms)
            .copied()
    }

    /// The ray at the start of the latest word that is exactly "there" (case and edge punctuation ignored).
    pub fn there_ray(&self, words: &[Word], max_gap_ms: u64) -> Option<(u64, Ray)> {
        let w = words.iter().rev().find(|w| {
            w.text.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase() == "there"
        })?;
        self.at(w.start_ms, max_gap_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ray turning steadily: direction encodes the sample time (dir[0] = t / 1000).
    fn ray(t_ms: u64) -> Ray {
        Ray { origin: [0.0, 0.0, -1.5], dir: [t_ms as f64 / 1000.0, 0.0, 0.0] }
    }

    fn hist() -> RayHistory {
        let mut h = RayHistory::new(10_000);
        for t in (0..=3000).step_by(10) {
            h.push(t, ray(t));
        }
        h
    }

    fn w(text: &str, start_ms: u64) -> Word {
        Word { text: text.into(), start_ms }
    }

    fn sentence(there: &str) -> Vec<Word> {
        vec![w("send", 1000), w("two", 1300), w("drones", 1600), w(there, 2000)]
    }

    #[test]
    fn there_picks_the_ray_at_the_word_start() {
        let (t, r) = hist().there_ray(&sentence("there"), 50).unwrap();
        assert_eq!(t, 2000);
        assert_eq!(r, ray(2000));
    }

    #[test]
    fn nearest_sample_when_between_samples() {
        let (t, _) = hist().at(2004, 50).unwrap();
        assert_eq!(t, 2000);
        let (t, _) = hist().at(2006, 50).unwrap();
        assert_eq!(t, 2010);
    }

    #[test]
    fn latest_there_wins() {
        let mut words = sentence("there");
        words.insert(1, w("there", 1100));
        words.push(w("there", 2500));
        assert_eq!(hist().there_ray(&words, 50).unwrap().0, 2500);
    }

    #[test]
    fn case_and_punctuation_match() {
        for text in ["There,", "there.", "THERE", "\"there\"!"] {
            assert_eq!(hist().there_ray(&sentence(text), 50).unwrap().0, 2000, "{text}");
        }
    }

    #[test]
    fn near_misses_do_not_match() {
        for text in ["therefore", "where", "their", "there's"] {
            assert!(hist().there_ray(&sentence(text), 50).is_none(), "{text}");
        }
    }

    #[test]
    fn no_ray_near_the_word_gives_none() {
        let mut h = RayHistory::new(10_000);
        h.push(1000, ray(1000));
        h.push(3000, ray(3000));
        assert!(h.there_ray(&sentence("there"), 100).is_none());
        assert_eq!(h.there_ray(&sentence("there"), 1000).unwrap().0, 1000);
        assert!(RayHistory::new(10_000).there_ray(&sentence("there"), 100).is_none());
    }

    #[test]
    fn word_older_than_the_window_gives_none() {
        let mut h = RayHistory::new(1000);
        for t in (0..=5000).step_by(10) {
            h.push(t, ray(t));
        }
        assert!(h.there_ray(&[w("there", 2000)], 50).is_none());
        assert_eq!(h.there_ray(&[w("there", 4500)], 50).unwrap().0, 4500);
    }

    #[test]
    fn out_of_order_pushes_are_ignored() {
        let mut h = RayHistory::new(10_000);
        h.push(100, ray(100));
        h.push(50, ray(50));
        assert!(h.at(50, 10).is_none());
        assert_eq!(h.at(100, 0).unwrap().1, ray(100));
    }

    #[test]
    fn window_drops_old_samples() {
        let mut h = RayHistory::new(100);
        h.push(0, ray(0));
        h.push(100, ray(100)); // 0 is exactly newest - window: kept
        assert!(h.at(0, 0).is_some());
        h.push(101, ray(101)); // now 0 is older than the window
        assert!(h.at(0, 0).is_none());
        assert!(h.at(100, 0).is_some());
    }
}
