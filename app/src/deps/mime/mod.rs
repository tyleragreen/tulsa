pub struct MimeType {
    source: Source,
    // For what we're doing, we don't technically need the Source type.
    // I'm keeping it in to see the library pattern but this would suffice
    // and mean we don't need the Source enum.
    // source: &'static str,
}

impl AsRef<str> for MimeType {
    fn as_ref(&self) -> &str {
        self.source.as_ref()
    }
}

enum Source {
    Atom(u8, &'static str),
    // We don't use this right now and I don't understand when you would need
    // it. Keeping it in case I get inspired in the future.
    _Dynamic(String),
}

impl Source {
    fn as_ref(&self) -> &str {
        match self {
            Source::Atom(_, s) => s,
            Source::_Dynamic(s) => s,
        }
    }
}

// Macro to simply the creation of a MimeType. This creates the boilerplate
// for creating a bunch of constants.
macro_rules! mime {
    ($name:ident, $s:expr, $len:expr) => {
        pub const $name: MimeType = MimeType {
            source: Source::Atom($len, $s),
        };
    };
}

// We only need APPLICATION_JSON right now but the rest of the types would
// go here.
mime! {
    // 11 seems to be the length of the string "application" but I'm not sure
    // why we have to specify it.
    APPLICATION_JSON, "application/json", 11
}
