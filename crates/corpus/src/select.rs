//! The manifest `codespec-corpus/1` and `select`, which derives it from a snapshot.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Error;
use crate::snapshot::{self, Snapshot};

/// The `format` of a manifest.
pub const FORMAT: &str = "codespec-corpus/1";

/// How `select` picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The first N by rank.
    Top,
    /// N by the seeded shuffle of the crate doc.
    Random,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Top => "top",
            Mode::Random => "random",
        }
    }
}

/// One repository of a manifest: nothing else about it is kept, and no source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repository {
    pub name: String,
    pub url: String,
    pub commit: String,
}

/// A corpus manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    /// The GitHub name of the language (`Go`).
    pub language: String,
    /// The `codespec read --lang` id (`go`).
    pub reader: String,
    pub mode: Mode,
    pub count: usize,
    /// The seed of `random`; `null` for `top`.
    pub seed: Option<u64>,
    /// The snapshot's `fetched_on`.
    pub fetched_on: String,
    /// The snapshot's [`Snapshot::digest`].
    pub snapshot_sha256: String,
    /// In rank order.
    pub repositories: Vec<Repository>,
}

impl Manifest {
    /// The bytes `select` writes: two-space indent, a final newline.
    pub fn canonical(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a manifest is JSON");
        text.push('\n');
        text
    }
}

/// splitmix64 (Vigna): state += 0x9E3779B97F4A7C15, then two xor-shift-multiply rounds with
/// 0xBF58476D1CE4E5B9 and 0x94D049BB133111EB, wrapping in 64 bits.
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// The indices `0..len` after the Fisher-Yates shuffle of the crate doc: `i` from `len - 1` down
/// to 1, `j = next_u64() % (i + 1)`, swap `i` and `j`.
pub fn shuffled(len: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..len).collect();
    let mut rng = SplitMix64::new(seed);
    for i in (1..len).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

/// The manifest of `snapshot` for `mode`, `count` and `seed`: a pure function of its arguments.
pub fn derive(
    snapshot: &Snapshot,
    mode: Option<Mode>,
    count: Option<usize>,
    seed: Option<u64>,
) -> Result<Manifest, Error> {
    let mode = mode.ok_or_else(|| Error("--mode: required (top or random)".to_owned()))?;
    let count = count.ok_or_else(|| Error("--count: required".to_owned()))?;
    match (mode, seed) {
        (Mode::Random, None) => {
            return Err(Error("--seed: required with --mode random".to_owned()));
        }
        (Mode::Top, Some(seed)) => {
            return Err(Error(format!(
                "--seed: {seed} is not allowed with --mode top, which takes no seed"
            )));
        }
        _ => {}
    }
    if count == 0 {
        return Err(Error("--count: 0 is below 1".to_owned()));
    }
    let rows = snapshot.by_rank();
    if count > rows.len() {
        return Err(Error(format!(
            "--count: {count} is more than the {} candidates of the snapshot",
            rows.len()
        )));
    }
    let (_, reader) = snapshot::language(&snapshot.language)?;
    let mut chosen: Vec<usize> = match seed {
        None => (0..count).collect(),
        Some(seed) => shuffled(rows.len(), seed)[..count].to_vec(),
    };
    chosen.sort_unstable();
    Ok(Manifest {
        format: FORMAT.to_owned(),
        language: snapshot.language.clone(),
        reader: reader.to_owned(),
        mode,
        count,
        seed,
        fetched_on: snapshot.fetched_on.clone(),
        snapshot_sha256: snapshot.digest(),
        repositories: chosen
            .into_iter()
            .map(|i| Repository {
                name: rows[i].name.clone(),
                url: rows[i].url.clone(),
                commit: rows[i].commit.clone(),
            })
            .collect(),
    })
}

/// Reads `manifest` strictly, derives it again from `snapshot` with the mode, count and seed it
/// records, and compares: the refusal names the first field that differs.
pub fn check(manifest: &[u8], snapshot: &Snapshot) -> Result<(), Error> {
    let recorded: Manifest =
        serde_json::from_slice(manifest).map_err(|e| Error(format!("manifest: {e}")))?;
    if recorded.format != FORMAT {
        return Err(Error(format!(
            "format: {:?} is not {FORMAT:?}",
            recorded.format
        )));
    }
    let derived = derive(
        snapshot,
        Some(recorded.mode),
        Some(recorded.count),
        recorded.seed,
    )
    .map_err(|e| {
        Error(format!(
            "manifest {} {}: {e}",
            recorded.mode.name(),
            recorded.count
        ))
    })?;
    let (recorded, derived) = (to_value(&recorded), to_value(&derived));
    for (field, want) in derived.as_object().expect("a manifest is an object") {
        if recorded.get(field) != Some(want) {
            return Err(Error(format!(
                "{field}: the manifest records {}, select derives {want} from the snapshot",
                recorded
                    .get(field)
                    .map_or("nothing".to_owned(), Value::to_string)
            )));
        }
    }
    Ok(())
}

fn to_value(manifest: &Manifest) -> Value {
    serde_json::to_value(manifest).expect("a manifest is JSON")
}

#[cfg(test)]
mod tests {
    use super::{SplitMix64, shuffled};

    #[test]
    fn splitmix64_matches_the_reference_vector() {
        let mut g = SplitMix64::new(0);
        assert_eq!(g.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(g.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(g.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn shuffle_is_a_permutation_and_fixed_by_the_seed() {
        let a = shuffled(50, 7);
        let mut sorted = a.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
        assert_eq!(a, shuffled(50, 7));
        assert_ne!(a, shuffled(50, 8));
        assert_eq!(shuffled(0, 1), Vec::<usize>::new());
        assert_eq!(shuffled(1, 1), vec![0]);
    }
}
