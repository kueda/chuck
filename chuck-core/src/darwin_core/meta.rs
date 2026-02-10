use std::fmt::Write;

use crate::darwin_core::{
    audiovisual::Audiovisual,
    comment::Comment,
    identification::Identification,
    multimedia::Multimedia,
    occurrence::Occurrence,
};
use chrono::Utc;

/// Metadata for the DarwinCore Archive
#[derive(Debug, Clone)]
pub struct Metadata {
    pub abstract_lines: Vec<String>,
}

/// Write `<field index="N" term="..."/>` elements
fn write_field_elements(xml: &mut String, fields: &[(&str, &str)]) {
    for (i, (_, term)) in fields.iter().enumerate() {
        writeln!(xml, r#"    <field index="{i}" term="{term}"/>"#).unwrap();
    }
}

/// Generates the meta.xml file for a DarwinCore Archive
pub fn generate_meta_xml(enabled_extensions: &[crate::DwcaExtension]) -> String {
    let mut xml = String::new();

    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://rs.tdwg.org/dwc/text/ http://rs.tdwg.org/dwc/text/tdwg_dwc_text.xsd">
"#);

    // <core>
    writeln!(
        xml,
        r#"  <core encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy="&quot;" ignoreHeaderLines="1" rowType="{}">
    <files>
      <location>{}</location>
    </files>
    <id index="0"/>"#,
        Occurrence::ROW_TYPE,
        Occurrence::FILENAME,
    )
    .unwrap();
    write_field_elements(&mut xml, Occurrence::WRITE_FIELDS);
    xml.push_str("  </core>\n");

    // Extensions
    let extension_specs = [
        (
            crate::DwcaExtension::SimpleMultimedia,
            Multimedia::ROW_TYPE,
            Multimedia::FILENAME,
            Multimedia::WRITE_FIELDS,
        ),
        (
            crate::DwcaExtension::Audiovisual,
            Audiovisual::ROW_TYPE,
            Audiovisual::FILENAME,
            Audiovisual::WRITE_FIELDS,
        ),
        (
            crate::DwcaExtension::Identifications,
            Identification::ROW_TYPE,
            Identification::FILENAME,
            Identification::WRITE_FIELDS,
        ),
        (
            crate::DwcaExtension::Comments,
            Comment::ROW_TYPE,
            Comment::FILENAME,
            Comment::WRITE_FIELDS,
        ),
    ];

    for (variant, row_type, filename, fields) in &extension_specs {
        if !enabled_extensions.contains(variant) {
            continue;
        }

        writeln!(
            xml,
            r#"  <extension encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy="&quot;" ignoreHeaderLines="1" rowType="{row_type}">
    <files>
      <location>{filename}</location>
    </files>
    <coreid index="0"/>"#
        )
        .unwrap();
        write_field_elements(&mut xml, fields);
        xml.push_str("  </extension>\n");
    }

    xml.push_str("</archive>\n");
    xml
}

// TODO: allow user to specify options like org name, contact info, license, etc
/// Generates an EML (Ecological Metadata Language) file for the archive
pub fn generate_eml(metadata: &Metadata) -> String {
    let now = Utc::now().format("%Y-%m-%d").to_string();
    let package_id =
        format!("darwincore-archive-{}", Utc::now().format("%Y%m%d%H%M%S"));

    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="eml://ecoinformatics.org/eml-2.1.1 http://rs.gbif.org/schema/eml-gbif-profile/1.1/eml.xsd"
  packageId="{package_id}"
  system="http://gbif.org"
  scope="system">
  <dataset>
    <title>Chuck DarwinCore Archive</title>
    <creator>
      <organizationName>Chuck</organizationName>
    </creator>
    <metadataProvider>
      <organizationName>Chuck</organizationName>
    </metadataProvider>
    <pubDate>{now}</pubDate>
    <language>en</language>
    <abstract>
"#
    );

    if metadata.abstract_lines.is_empty() {
        xml.push_str(
            "      <para>Observations exported from iNaturalist</para>\n",
        );
    } else {
        for line in &metadata.abstract_lines {
            let escaped = xml_escape(line);
            writeln!(xml, "      <para>{escaped}</para>").unwrap();
        }
    }

    xml.push_str(
        r#"    </abstract>
    <contact>
      <organizationName>Chuck</organizationName>
    </contact>
  </dataset>
</eml:eml>
"#,
    );

    xml
}

/// Escape special XML characters in text content
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::darwin_core::occurrence::Occurrence;

    fn core_field_names(meta_xml: &str) -> Vec<String> {
        let doc = roxmltree::Document::parse(meta_xml).unwrap();
        let core = doc.descendants()
            .find(|n| n.has_tag_name("core"))
            .expect("no <core> element");
        let mut fields: Vec<(usize, String)> = core.children()
            .filter(|n| n.has_tag_name("field"))
            .map(|n| {
                let idx: usize = n.attribute("index").unwrap().parse().unwrap();
                let term = n.attribute("term").unwrap();
                let short = term.rsplit('/').next().unwrap().to_string();
                (idx, short)
            })
            .collect();
        fields.sort_by_key(|(i, _)| *i);
        fields.into_iter().map(|(_, name)| name).collect()
    }

    #[test]
    fn test_csv_headers_match_meta_xml_core_fields() {
        let meta_xml = generate_meta_xml(&[]);
        let field_names = core_field_names(&meta_xml);
        let headers = Occurrence::csv_headers();

        let _ = env_logger::try_init();
        log::debug!("field_names: {:?}", field_names);
        log::debug!("csv headers: {:?}", headers);

        assert_eq!(
            headers.len(), field_names.len(),
            "csv_headers has {} columns but meta.xml has {} fields",
            headers.len(), field_names.len(),
        );

        for (i, (header, field)) in headers.iter().zip(&field_names).enumerate() {
            assert_eq!(
                *header, field,
                "column {i}: csv_headers has \"{header}\" but meta.xml has \"{field}\"",
            );
        }
    }
}
