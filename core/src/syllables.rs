//! Split Khmer words into written clusters and spoken syllables, ported from khmer-engine's
//! `syllables.py`.
//!
//! A cluster is what Khmer writes as one unit: a base letter, its subscripts, a vowel and
//! signs. A syllable is what is pronounced. They differ because the final consonant of one
//! syllable is often written as the base of the next cluster, with the next onset written
//! under it: in កម្ពុជា the ម closes "kam" and the ព under it starts "pu".
//!
//! Which reading is meant is not marked in the script, so [`syllables`] scores the
//! possible readings and keeps the cheapest. The costs encode a few preferences:
//!
//! - a bare consonant closes the open syllable before it rather than starting its own
//!   syllable with an inherent vowel (កក is "kak", not "ka-ka");
//! - a doubled consonant such as ប្ប is split across two syllables (សប្បាយ is "sab-bay");
//! - a word does not end in a bare consonant with an inherent vowel (បេក្ខជន ends "chon").

use crate::script::{
    self, AHSDA, BANTOC, COENG, KAKABAT, MUUSIKATOAN, NIKAHIT, REAHMUK, ROBAT, Series, TOANDAKHIAT,
    TRIISAP, VIRIAM, YUUKALEAPINTU, is_consonant, is_dependent_vowel, is_independent_vowel,
    is_series_neutral, is_shifter, is_sign, is_vocalic_sign,
};
use crate::segment::is_khmer_letter;

/// One written cluster. `base` is `None` when vowel signs appear without a letter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cluster {
    pub text: String,
    pub base: Option<char>,
    pub subscripts: Vec<char>,
    pub shifter: Option<char>,
    pub vowel: String,
    pub signs: String,
}

impl Cluster {
    pub fn is_independent_vowel(&self) -> bool {
        self.base.is_some_and(is_independent_vowel)
    }

    /// Does the cluster carry its own vowel (so it cannot be a bare final)?
    pub fn has_nucleus(&self) -> bool {
        self.base.is_none()
            || self.is_independent_vowel()
            || !self.vowel.is_empty()
            || self.signs.chars().any(is_vocalic_sign)
            // ហ្ឫទ័យ: an independent vowel written as a subscript is the vowel.
            || self.subscripts.iter().any(|&sub| is_independent_vowel(sub))
    }

    /// Bantoc and viriam only sit on final consonants.
    pub fn must_be_final(&self) -> bool {
        self.signs.contains(BANTOC) || self.signs.contains(VIRIAM)
    }

    /// The base and its subscripts; nothing when there is no base.
    fn letters(&self) -> Vec<char> {
        match self.base {
            Some(base) => std::iter::once(base)
                .chain(self.subscripts.iter().copied())
                .collect(),
            None => Vec::new(),
        }
    }
}

/// Split normalized Khmer text into clusters. Characters outside words are skipped.
pub fn clusters(text: &str) -> Vec<Cluster> {
    let mut out = Vec::new();
    let mut current: Option<Cluster> = None;
    let mut after_coeng = false;
    for ch in text.chars() {
        let is_letter = is_consonant(ch) || is_independent_vowel(ch);
        if is_letter
            && after_coeng
            && let Some(cluster) = current.as_mut()
        {
            cluster.subscripts.push(ch);
            cluster.text.push(ch);
            after_coeng = false;
            continue;
        }
        after_coeng = false;
        if is_letter {
            out.extend(current.replace(Cluster {
                text: ch.to_string(),
                base: Some(ch),
                ..Cluster::default()
            }));
            continue;
        }
        if !is_khmer_letter(ch) {
            continue;
        }
        let cluster = current.get_or_insert_with(Cluster::default);
        cluster.text.push(ch);
        if ch == COENG {
            after_coeng = true;
        } else if is_shifter(ch) {
            cluster.shifter = Some(ch);
        } else if is_dependent_vowel(ch) {
            cluster.vowel.push(ch);
        } else if is_sign(ch) {
            cluster.signs.push(ch);
        }
    }
    out.extend(current);
    out
}

/// One spoken syllable.
///
/// `onset` holds the onset consonants in writing order and is empty when the syllable
/// starts with an independent vowel (`independent`). `subscript_onset` is true when the
/// onset is written under the previous syllable's final consonant. `signs` holds the
/// signs of the nucleus plus bantoc when it sits on the final. `silent` marks a nucleus
/// written with toandakhiat (not pronounced), `silent_finals` the same for the finals.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the same flags as the engine's Syllable"
)]
pub struct Syllable {
    pub text: String,
    pub onset: Vec<char>,
    pub independent: Option<char>,
    pub shifter: Option<char>,
    pub vowel: String,
    pub signs: String,
    pub finals: Vec<char>,
    pub subscript_onset: bool,
    pub robat: bool,
    pub silent: bool,
    pub silent_finals: bool,
}

impl Syllable {
    pub fn bantoc(&self) -> bool {
        self.signs.contains(BANTOC)
    }

    /// Series of the nucleus: from the onset consonants and shifter (UNGEGN notes 2-3).
    pub fn series(&self) -> Series {
        let Some(mut result) = self.onset.first().and_then(|&first| script::series(first)) else {
            return Series::A;
        };
        if let Some(&subscript) = self.onset.get(1)
            && !is_series_neutral(subscript)
            && let Some(series) = script::series(subscript)
        {
            result = series;
        }
        match self.shifter {
            Some(MUUSIKATOAN) => Series::A,
            Some(TRIISAP) => Series::O,
            _ => result,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Start,
    /// A bare onset, no final yet.
    OpenInherent,
    /// An explicit vowel, no final yet.
    OpenVowel,
    /// ាំ, which can still take ង as a final (ាំង).
    OpenAam,
    Closed,
}

impl State {
    fn is_open(self) -> bool {
        matches!(self, State::OpenInherent | State::OpenVowel)
    }
}

/// How a cluster is read. The order is the tie-break: earlier actions win when costs are
/// equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Action {
    /// The cluster is the final consonant(s) of the open syllable.
    Final,
    /// The base closes the open syllable; the subscripts start the next one.
    Split,
    /// The cluster starts a new syllable.
    Onset,
}

/// A syllable with an inherent vowel and no final.
const LEFT_OPEN_COST: f64 = 1.0;
/// The same at the end of a word.
const WORD_END_OPEN_COST: f64 = 2.0;
/// សាស្ត្រ: prefer a new onset after an explicit vowel.
const SPLIT_AFTER_VOWEL_COST: f64 = 0.5;
/// ប្ប as an onset.
const DOUBLED_ONSET_COST: f64 = 2.0;
/// Bantoc on an onset.
const MISPLACED_FINAL_COST: f64 = 5.0;

fn after_nucleus(cluster: &Cluster) -> State {
    let signs = &cluster.signs;
    if signs.contains(NIKAHIT) {
        return if cluster.vowel == "ា" {
            State::OpenAam
        } else {
            State::Closed
        };
    }
    if [REAHMUK, YUUKALEAPINTU, AHSDA, KAKABAT]
        .iter()
        .any(|&sign| signs.contains(sign))
    {
        return State::Closed;
    }
    State::OpenVowel
}

/// Every legal reading of `cluster` after a syllable in `state`, with its cost and the
/// state it leaves.
fn options(cluster: &Cluster, state: State, last: bool) -> Vec<(Action, f64, State)> {
    let doubled = match (cluster.base, cluster.subscripts.first()) {
        (Some(base), Some(&first)) if base == first => DOUBLED_ONSET_COST,
        _ => 0.0,
    };
    let left_open = if state == State::OpenInherent {
        LEFT_OPEN_COST
    } else {
        0.0
    };
    let can_split =
        !cluster.subscripts.is_empty() && state.is_open() && !cluster.is_independent_vowel();
    let split_cost = if state == State::OpenInherent {
        0.0
    } else {
        SPLIT_AFTER_VOWEL_COST
    };
    let mut out = Vec::new();
    if cluster.has_nucleus() {
        if can_split {
            out.push((Action::Split, split_cost, after_nucleus(cluster)));
        }
        out.push((Action::Onset, doubled + left_open, after_nucleus(cluster)));
        return out;
    }
    let takes_final = state.is_open()
        || (state == State::OpenAam && cluster.base == Some('ង') && cluster.subscripts.is_empty());
    if takes_final && (cluster.subscripts.is_empty() || last) {
        out.push((Action::Final, 0.0, State::Closed));
    }
    if can_split {
        out.push((Action::Split, 0.0, State::OpenInherent));
    }
    let misplaced = if cluster.must_be_final() {
        MISPLACED_FINAL_COST
    } else {
        0.0
    };
    out.push((
        Action::Onset,
        doubled + left_open + misplaced,
        State::OpenInherent,
    ));
    out
}

/// The cheapest reading of every cluster: a Viterbi search over (state, whether there is
/// more than one syllable so far). Hypotheses keep the engine's order, so that ties are
/// broken the same way.
fn best_actions(clusters: &[Cluster]) -> Vec<Action> {
    type Hypothesis = ((State, bool), f64, Vec<Action>);
    let mut best: Vec<Hypothesis> = vec![((State::Start, false), 0.0, Vec::new())];
    for (i, cluster) in clusters.iter().enumerate() {
        let last = i == clusters.len() - 1;
        let mut next: Vec<Hypothesis> = Vec::new();
        for ((state, several), cost, actions) in &best {
            for (action, step, new_state) in options(cluster, *state, last) {
                let key = (
                    new_state,
                    *several || (action != Action::Final && *state != State::Start),
                );
                let total = cost + step;
                let extended = || [actions.as_slice(), &[action]].concat();
                match next.iter_mut().find(|(k, _, _)| *k == key) {
                    Some(entry) if total < entry.1 => (entry.1, entry.2) = (total, extended()),
                    Some(_) => {}
                    None => next.push((key, total, extended())),
                }
            }
        }
        best = next;
    }
    best.into_iter()
        .map(|((state, several), cost, actions)| {
            let end = if state == State::OpenInherent && several {
                WORD_END_OPEN_COST
            } else {
                0.0
            };
            (cost + end, actions)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, actions)| actions)
        .unwrap_or_default()
}

/// Start a syllable at `cluster`, whose onset letters are `onset` (written under the
/// previous final when `subscript` is true).
fn start(cluster: &Cluster, text: &str, onset: &[char], subscript: bool) -> Syllable {
    let (independent, onset) = match (cluster.base, onset.split_first()) {
        (Some(base), _) if is_independent_vowel(base) && !subscript => (Some(base), &[][..]),
        // អម្ឫត: ឫ starts the second syllable.
        (_, Some((&first, rest))) if is_independent_vowel(first) => (Some(first), rest),
        _ => (None, onset),
    };
    Syllable {
        text: text.to_owned(),
        onset: onset.to_vec(),
        independent,
        shifter: cluster.shifter,
        vowel: cluster.vowel.clone(),
        signs: cluster
            .signs
            .chars()
            .filter(|&s| is_vocalic_sign(s))
            .collect(),
        subscript_onset: subscript,
        robat: cluster.signs.contains(ROBAT) && !subscript,
        silent: cluster.signs.contains(TOANDAKHIAT) && cluster.has_nucleus(),
        ..Syllable::default()
    }
}

/// Add final consonants, with the signs written on them, to an open syllable.
fn close(syllable: &mut Syllable, letters: &[char], text: &str, signs: &str) {
    syllable.text.push_str(text);
    syllable.finals.extend_from_slice(letters);
    if signs.contains(BANTOC) {
        syllable.signs.push(BANTOC);
    }
    if signs.contains(ROBAT) {
        syllable.robat = true;
    }
    if signs.contains(TOANDAKHIAT) {
        syllable.silent_finals = true;
    }
}

/// Split a Khmer word into syllables. The word is normalized first with pheasa's defaults,
/// as in the engine.
pub fn syllables(word: &str) -> Vec<Syllable> {
    let clusters = clusters(&pheasa::normalize(word));
    let mut out: Vec<Syllable> = Vec::new();
    for (cluster, action) in clusters.iter().zip(best_actions(&clusters)) {
        // Only an open syllable takes a final or a split base, so `out` has one.
        match (action, out.last_mut()) {
            (Action::Final, Some(previous)) => {
                close(previous, &cluster.letters(), &cluster.text, &cluster.signs);
            }
            (Action::Split, Some(previous)) => {
                // Only robat belongs to the base; every other sign goes with the new onset.
                let cut = cluster
                    .text
                    .find(COENG)
                    .expect("a subscript follows a coeng");
                let robat: String = cluster.signs.chars().filter(|&s| s == ROBAT).collect();
                let base: Vec<char> = cluster.base.into_iter().collect();
                close(previous, &base, &cluster.text[..cut], &robat);
                out.push(start(
                    cluster,
                    &cluster.text[cut..],
                    &cluster.subscripts,
                    true,
                ));
            }
            _ => out.push(start(cluster, &cluster.text, &cluster.letters(), false)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Series, clusters, syllables};

    fn texts(word: &str) -> Vec<String> {
        syllables(word).into_iter().map(|s| s.text).collect()
    }

    #[test]
    fn clusters_split_on_base_letters() {
        let parts = clusters("ក្រុមហ៊ុន");
        let texts: Vec<&str> = parts.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(texts, ["ក្រុ", "ម", "ហ៊ុ", "ន"]);
        assert_eq!(parts[0].base, Some('ក'));
        assert_eq!(parts[0].subscripts, ['រ']);
        assert_eq!(parts[0].vowel, "ុ");
        assert_eq!(parts[2].shifter, Some('៊'));
    }

    #[test]
    fn clusters_skip_text_outside_words() {
        let texts: Vec<String> = clusters("ក a ខ").into_iter().map(|c| c.text).collect();
        assert_eq!(texts, ["ក", "ខ"]);
    }

    #[test]
    fn vowel_without_base_gets_its_own_cluster() {
        let parts = clusters("ា");
        assert_eq!(parts.len(), 1);
        assert_eq!((parts[0].base, parts[0].vowel.as_str()), (None, "ា"));
    }

    #[test]
    fn syllable_boundaries() {
        let cases: [(&str, &[&str]); 21] = [
            // A bare consonant closes the syllable before it.
            ("កក", &["កក"]),
            ("ធនាគារ", &["ធ", "នា", "គារ"]),
            ("អរគុណ", &["អរ", "គុណ"]),
            // The base of a cluster with a subscript closes the syllable before it.
            ("កម្ពុជា", &["កម", "្ពុ", "ជា"]),
            ("កន្ត្រាប់", &["កន", "្ត្រាប់"]),
            ("ចង្អៀត", &["ចង", "្អៀត"]),
            ("អក្សរខ្មែរ", &["អក", "្សរ", "ខ្មែរ"]),
            // A doubled consonant is split across syllables.
            ("សប្បាយ", &["សប", "្បាយ"]),
            ("ទស្សនា", &["ទស", "្ស", "នា"]),
            // Word-final clusters with subscripts are finals.
            ("អង្គ", &["អង្គ"]),
            ("បុណ្យ", &["បុណ្យ"]),
            ("ភ័ព្វ", &["ភ័ព្វ"]),
            // The word does not end on a bare consonant with an inherent vowel.
            ("បេក្ខជន", &["បេក", "្ខ", "ជន"]),
            // Nikahit closes a syllable, except that ាំ still takes ង.
            ("ដំបង", &["ដំ", "បង"]),
            ("ទាំង", &["ទាំង"]),
            // Initial clusters start a syllable.
            ("ក្រចេះ", &["ក្រ", "ចេះ"]),
            ("ស្ត្រី", &["ស្ត្រី"]),
            ("ស្ទឹងត្រែង", &["ស្ទឹង", "ត្រែង"]),
            // Independent vowels.
            ("ឪពុក", &["ឪ", "ពុក"]),
            ("ឥឡូវ", &["ឥ", "ឡូវ"]),
            // Coeng da is read as coeng ta, like the rest of the core's input.
            ("កណ្ដាល", &["កណ", "្តាល"]),
        ];
        for (word, expected) in cases {
            assert_eq!(texts(word), expected, "{word}");
        }
    }

    #[test]
    fn split_onset_comes_from_the_subscript() {
        let parts = syllables("កម្ពុជា");
        assert_eq!(parts[0].finals, ['ម']);
        assert_eq!(parts[1].onset, ['ព']);
        assert!(parts[1].subscript_onset);
        assert_eq!(parts[1].vowel, "ុ");
    }

    #[test]
    fn bantoc_is_recorded_on_the_syllable_it_closes() {
        let parts = syllables("ចាក់");
        assert_eq!(parts.len(), 1);
        assert!(parts[0].bantoc());
        assert_eq!(parts[0].finals, ['ក']);
    }

    #[test]
    fn robat_on_a_final_marks_its_syllable() {
        let parts = syllables("ធម៌");
        assert_eq!(parts.len(), 1);
        assert!(parts[0].robat);
        assert_eq!(parts[0].finals, ['ម']);
    }

    #[test]
    fn toandakhiat_marks_a_silent_syllable() {
        let silent: Vec<bool> = syllables("ពោធិ៍សាត់").iter().map(|s| s.silent).collect();
        assert_eq!(silent, [false, true, false]);
    }

    #[test]
    fn series() {
        let cases = [
            ("ក", Series::A),
            ("គ", Series::O),
            ("ខ្ពង", Series::O), // a non-sonorant subscript decides
            ("ល្អ", Series::A),
            ("ស្វាយ", Series::A), // វ is series-neutral, so the base decides
            ("ខ្ញុំ", Series::A),
            ("ហ៊ាង", Series::O), // triisap
            ("ញ៉ាំ", Series::A),  // muusikatoan
            ("ឯក", Series::A),
        ];
        for (word, expected) in cases {
            assert_eq!(syllables(word)[0].series(), expected, "{word}");
        }
    }

    #[test]
    fn empty_word() {
        assert_eq!(syllables(""), Vec::new());
    }

    #[test]
    fn independent_vowel_as_subscript_is_a_nucleus() {
        let parts = syllables("ហ្ឫទ័យ");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].onset, ['ហ', 'ឫ']);
        assert_eq!(parts[0].series(), Series::A);
        assert_eq!(parts[1].finals, ['យ']);
    }

    #[test]
    fn subscript_independent_vowel_can_start_a_syllable() {
        let parts = syllables("អម្ឫត");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].finals, ['ម']);
        assert_eq!(parts[1].onset, Vec::<char>::new());
        assert_eq!(parts[1].independent, Some('ឫ'));
        assert_eq!(parts[1].finals, ['ត']);
    }
}
