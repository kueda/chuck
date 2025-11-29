#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DwcaExtension {
    /// Simple Multimedia extension
    SimpleMultimedia,
    /// Audiovisual Media Description extension
    Audiovisual,
    /// Identifications extension
    Identifications,
}

impl DwcaExtension {
    /// Convert from a rowType URL to a DwcaExtension variant
    pub fn from_row_type(row_type: &str) -> Option<Self> {
        match row_type {
            "http://rs.gbif.org/terms/1.0/Multimedia" => Some(Self::SimpleMultimedia),
            "http://rs.tdwg.org/ac/terms/Multimedia" => Some(Self::Audiovisual),
            "http://rs.tdwg.org/dwc/terms/Identification" => Some(Self::Identifications),
            _ => None,
        }
    }

    /// Get the underscored table name for this extension
    pub fn table_name(&self) -> &'static str {
        match self {
            Self::SimpleMultimedia => "multimedia",
            Self::Audiovisual => "audiovisual",
            Self::Identifications => "identifications",
        }
    }

    /// Get all supported row types
    pub const fn all_row_types() -> &'static [&'static str] {
        &[
            "http://rs.gbif.org/terms/1.0/Multimedia",
            "http://rs.tdwg.org/ac/terms/Multimedia",
            "http://rs.tdwg.org/dwc/terms/Identification",
        ]
    }
}

impl std::fmt::Display for DwcaExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SimpleMultimedia => write!(f, "SimpleMultimedia"),
            Self::Audiovisual => write!(f, "Audiovisual"),
            Self::Identifications => write!(f, "Identifications"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_row_type() {
        assert_eq!(
            DwcaExtension::from_row_type("http://rs.gbif.org/terms/1.0/Multimedia"),
            Some(DwcaExtension::SimpleMultimedia)
        );
        assert_eq!(
            DwcaExtension::from_row_type("http://rs.tdwg.org/ac/terms/Multimedia"),
            Some(DwcaExtension::Audiovisual)
        );
        assert_eq!(
            DwcaExtension::from_row_type("http://rs.tdwg.org/dwc/terms/Identification"),
            Some(DwcaExtension::Identifications)
        );
        assert_eq!(
            DwcaExtension::from_row_type("http://unknown.org/Unknown"),
            None
        );
    }

    #[test]
    fn test_table_name() {
        assert_eq!(DwcaExtension::SimpleMultimedia.table_name(), "multimedia");
        assert_eq!(DwcaExtension::Audiovisual.table_name(), "audiovisual");
        assert_eq!(DwcaExtension::Identifications.table_name(), "identifications");
    }

    #[test]
    fn test_all_row_types() {
        let row_types = DwcaExtension::all_row_types();
        assert_eq!(row_types.len(), 3);
        assert!(row_types.contains(&"http://rs.gbif.org/terms/1.0/Multimedia"));
        assert!(row_types.contains(&"http://rs.tdwg.org/ac/terms/Multimedia"));
        assert!(row_types.contains(&"http://rs.tdwg.org/dwc/terms/Identification"));
    }
}
