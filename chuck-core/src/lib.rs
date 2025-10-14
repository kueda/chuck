pub mod api;
pub mod auth;
pub mod darwin_core;

#[derive(Clone, Debug, PartialEq)]
pub enum DwcExtension {
    /// Simple Multimedia extension
    SimpleMultimedia,
    /// Audiovisual Media Description extension
    Audiovisual,
    /// Identifications extension
    Identifications,
}

impl std::fmt::Display for DwcExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SimpleMultimedia => write!(f, "SimpleMultimedia"),
            Self::Audiovisual => write!(f, "Audiovisual"),
            Self::Identifications => write!(f, "Identifications"),
        }
    }
}
