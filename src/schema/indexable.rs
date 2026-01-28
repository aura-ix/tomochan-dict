use super::{Kanji, KanjiMeta, Tag, Term, TermMeta};

pub trait Indexable {
    fn lookup_key(&self) -> String;
}

impl Indexable for Term {
    fn lookup_key(&self) -> String {
        self.term.clone()
    }
}

impl Indexable for Kanji {
    fn lookup_key(&self) -> String {
        self.character.clone()
    }
}

impl Indexable for Tag {
    fn lookup_key(&self) -> String {
        self.name.clone()
    }
}

impl Indexable for TermMeta {
    fn lookup_key(&self) -> String {
        self.term.clone()
    }
}

impl Indexable for KanjiMeta {
    fn lookup_key(&self) -> String {
        self.character.clone()
    }
}
