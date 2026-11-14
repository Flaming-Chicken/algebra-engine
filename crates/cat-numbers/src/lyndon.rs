//! Lyndon word basis and Shirshov bracket factorization for free Lie algebras.

use smallvec::SmallVec;

/// Lyndon word representation over an alphabet of generator indices 0..n.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LyndonWord(pub SmallVec<[u8; 8]>);

impl LyndonWord {
    /// Create word from generator index.
    pub fn generator(generator_idx: u8) -> Self {
        Self(smallvec::smallvec![generator_idx])
    }

    /// Check if a word is a valid Lyndon word (strictly smaller than all non-trivial proper suffixes).
    pub fn is_lyndon(letters: &[u8]) -> bool {
        if letters.is_empty() {
            return false;
        }
        if letters.len() == 1 {
            return true;
        }
        for i in 1..letters.len() {
            if &letters[i..] <= letters {
                return false;
            }
        }
        true
    }

    /// Canonical Shirshov factorization: w = u v where u, v are Lyndon words and v has maximum length.
    pub fn shirshov_factorization(&self) -> Option<(LyndonWord, LyndonWord)> {
        let n = self.0.len();
        if n < 2 {
            return None;
        }

        // Find longest proper suffix that is a Lyndon word
        for i in (1..n).rev() {
            let suffix = &self.0[i..];
            let prefix = &self.0[..i];
            if Self::is_lyndon(suffix) && Self::is_lyndon(prefix) {
                return Some((
                    LyndonWord(SmallVec::from_slice(prefix)),
                    LyndonWord(SmallVec::from_slice(suffix)),
                ));
            }
        }
        None
    }

    /// Length of the Lyndon word (degree in the free Lie algebra).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the word has length 0.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Free Lie algebra bracket element in the Lyndon basis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LieBracketTree {
    Generator(u8),
    Bracket(Box<LieBracketTree>, Box<LieBracketTree>),
}

impl LieBracketTree {
    /// Construct standard Lyndon bracket via recursive Shirshov factorization.
    pub fn from_lyndon(word: &LyndonWord) -> Self {
        if word.len() == 1 {
            Self::Generator(word.0[0])
        } else if let Some((u, v)) = word.shirshov_factorization() {
            Self::Bracket(
                Box::new(Self::from_lyndon(&u)),
                Box::new(Self::from_lyndon(&v)),
            )
        } else {
            Self::Generator(word.0[0])
        }
    }
}
