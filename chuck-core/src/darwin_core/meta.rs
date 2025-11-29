use chrono::Utc;

/// Metadata for the DarwinCore Archive
#[derive(Debug, Clone)]
pub struct Metadata {
    pub abstract_lines: Vec<String>,
}

/// Generates the meta.xml file for a DarwinCore Archive
pub fn generate_meta_xml(enabled_extensions: &[crate::DwcaExtension]) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"

         xsi:schemaLocation="http://rs.tdwg.org/dwc/text/ http://rs.tdwg.org/dwc/text/tdwg_dwc_text.xsd">
  <core encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy='"' ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/basisOfRecord"/>
    <field index="2" term="http://rs.tdwg.org/dwc/terms/recordedBy"/>
    <field index="3" term="http://rs.tdwg.org/dwc/terms/eventDate"/>
    <field index="4" term="http://rs.tdwg.org/dwc/terms/decimalLatitude"/>
    <field index="5" term="http://rs.tdwg.org/dwc/terms/decimalLongitude"/>
    <field index="6" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
    <field index="7" term="http://rs.tdwg.org/dwc/terms/taxonRank"/>
    <field index="8" term="http://rs.tdwg.org/dwc/terms/taxonomicStatus"/>
    <field index="9" term="http://rs.tdwg.org/dwc/terms/vernacularName"/>
    <field index="10" term="http://rs.tdwg.org/dwc/terms/kingdom"/>
    <field index="11" term="http://rs.tdwg.org/dwc/terms/phylum"/>
    <field index="12" term="http://rs.tdwg.org/dwc/terms/class"/>
    <field index="13" term="http://rs.tdwg.org/dwc/terms/order"/>
    <field index="14" term="http://rs.tdwg.org/dwc/terms/family"/>
    <field index="15" term="http://rs.tdwg.org/dwc/terms/genus"/>
    <field index="16" term="http://rs.tdwg.org/dwc/terms/specificEpithet"/>
    <field index="17" term="http://rs.tdwg.org/dwc/terms/infraspecificEpithet"/>
    <field index="18" term="http://rs.tdwg.org/dwc/terms/occurrenceRemarks"/>
    <field index="19" term="http://rs.tdwg.org/dwc/terms/establishmentMeans"/>
    <field index="20" term="http://rs.tdwg.org/dwc/terms/georeferencedDate"/>
    <field index="21" term="http://rs.tdwg.org/dwc/terms/georeferenceProtocol"/>
    <field index="22" term="http://rs.tdwg.org/dwc/terms/coordinateUncertaintyInMeters"/>
    <field index="23" term="http://rs.tdwg.org/dwc/terms/coordinatePrecision"/>
    <field index="24" term="http://rs.tdwg.org/dwc/terms/geodeticDatum"/>
    <field index="25" term="http://purl.org/dc/terms/accessRights"/>
    <field index="26" term="http://purl.org/dc/terms/license"/>
    <field index="27" term="http://rs.tdwg.org/dwc/terms/informationWithheld"/>
    <field index="28" term="http://purl.org/dc/terms/modified"/>
    <field index="29" term="https://www.inaturalist.org/terminology/captive_cultivated"/>
    <field index="30" term="http://rs.tdwg.org/dwc/terms/eventTime"/>
    <field index="31" term="http://rs.tdwg.org/dwc/terms/verbatimEventDate"/>
  </core>{extensions}
</archive>"#, extensions = generate_extensions(enabled_extensions))
}

/// Generate extension XML for enabled extensions
fn generate_extensions(enabled_extensions: &[crate::DwcaExtension]) -> String {
    let mut extensions = String::new();

    if enabled_extensions.contains(&crate::DwcaExtension::SimpleMultimedia) {
        extensions.push_str(r#"
  <extension encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy='"' ignoreHeaderLines="1" rowType="http://rs.gbif.org/terms/1.0/Multimedia">
    <files>
      <location>multimedia.csv</location>
    </files>
    <coreid index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://purl.org/dc/terms/type"/>
    <field index="2" term="http://purl.org/dc/terms/format"/>
    <field index="3" term="http://purl.org/dc/terms/identifier"/>
    <field index="4" term="http://purl.org/dc/terms/references"/>
    <field index="5" term="http://purl.org/dc/terms/title"/>
    <field index="6" term="http://purl.org/dc/terms/description"/>
    <field index="7" term="http://purl.org/dc/terms/created"/>
    <field index="8" term="http://purl.org/dc/terms/creator"/>
    <field index="9" term="http://purl.org/dc/terms/contributor"/>
    <field index="10" term="http://purl.org/dc/terms/publisher"/>
    <field index="11" term="http://purl.org/dc/terms/audience"/>
    <field index="12" term="http://purl.org/dc/terms/source"/>
    <field index="13" term="http://purl.org/dc/terms/license"/>
    <field index="14" term="http://purl.org/dc/terms/rightsHolder"/>
    <field index="15" term="http://rs.tdwg.org/dwc/terms/datasetID"/>
  </extension>"#);
    }

    if enabled_extensions.contains(&crate::DwcaExtension::Audiovisual) {
        extensions.push_str(r#"
  <extension encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy='"' ignoreHeaderLines="1" rowType="http://rs.tdwg.org/ac/terms/Multimedia">
    <files>
      <location>audiovisual.csv</location>
    </files>
    <coreid index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://purl.org/dc/terms/identifier"/>
    <field index="2" term="http://purl.org/dc/terms/type"/>
    <field index="3" term="http://purl.org/dc/terms/title"/>
    <field index="4" term="http://purl.org/dc/terms/modified"/>
    <field index="5" term="http://rs.tdwg.org/ac/terms/metadataLanguageLiteral"/>
    <field index="6" term="http://purl.org/dc/terms/available"/>
    <field index="7" term="http://purl.org/dc/terms/rights"/>
    <field index="8" term="http://ns.adobe.com/xap/1.0/rights/Owner"/>
    <field index="9" term="http://ns.adobe.com/xap/1.0/rights/UsageTerms"/>
    <field index="10" term="http://ns.adobe.com/photoshop/1.0/Credit"/>
    <field index="11" term="http://rs.tdwg.org/ac/terms/attributionLinkURL"/>
    <field index="12" term="http://purl.org/dc/terms/source"/>
    <field index="13" term="http://purl.org/dc/terms/description"/>
    <field index="14" term="http://rs.tdwg.org/ac/terms/caption"/>
    <field index="15" term="http://rs.tdwg.org/ac/terms/comments"/>
    <field index="16" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
    <field index="17" term="http://rs.tdwg.org/ac/terms/commonName"/>
    <field index="18" term="http://rs.tdwg.org/dwc/terms/lifeStage"/>
    <field index="19" term="http://rs.tdwg.org/ac/terms/partOfOrganism"/>
    <field index="20" term="http://rs.tdwg.org/ac/terms/locationShown"/>
    <field index="21" term="http://rs.tdwg.org/ac/terms/locationCreated"/>
    <field index="22" term="http://rs.tdwg.org/dwc/terms/continent"/>
    <field index="23" term="http://rs.tdwg.org/dwc/terms/country"/>
    <field index="24" term="http://rs.tdwg.org/dwc/terms/countryCode"/>
    <field index="25" term="http://rs.tdwg.org/dwc/terms/stateProvince"/>
    <field index="26" term="http://rs.tdwg.org/dwc/terms/locality"/>
    <field index="27" term="http://rs.tdwg.org/dwc/terms/decimalLatitude"/>
    <field index="28" term="http://rs.tdwg.org/dwc/terms/decimalLongitude"/>
    <field index="29" term="http://rs.tdwg.org/ac/terms/accessURI"/>
    <field index="30" term="http://purl.org/dc/terms/format"/>
    <field index="31" term="http://purl.org/dc/terms/extent"/>
    <field index="32" term="http://rs.tdwg.org/ac/terms/pixelXDimension"/>
    <field index="33" term="http://rs.tdwg.org/ac/terms/pixelYDimension"/>
    <field index="34" term="http://purl.org/dc/terms/created"/>
    <field index="35" term="http://rs.tdwg.org/ac/terms/dateTimeOriginal"/>
    <field index="36" term="http://purl.org/dc/terms/temporal"/>
  </extension>"#);
    }

    if enabled_extensions.contains(&crate::DwcaExtension::Identifications) {
        extensions.push_str(r#"
  <extension encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy='"' ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Identification">
    <files>
      <location>identification.csv</location>
    </files>
    <coreid index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/identificationID"/>
    <field index="2" term="http://rs.tdwg.org/dwc/terms/identifiedBy"/>
    <field index="3" term="http://rs.tdwg.org/dwc/terms/identifiedByID"/>
    <field index="4" term="http://rs.tdwg.org/dwc/terms/dateIdentified"/>
    <field index="5" term="http://rs.tdwg.org/dwc/terms/identificationRemarks"/>
    <field index="6" term="http://rs.tdwg.org/dwc/terms/taxonID"/>
    <field index="7" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
    <field index="8" term="http://rs.tdwg.org/dwc/terms/taxonRank"/>
    <field index="9" term="http://rs.tdwg.org/dwc/terms/vernacularName"/>
    <field index="10" term="http://rs.tdwg.org/dwc/terms/taxonomicStatus"/>
    <field index="11" term="http://rs.tdwg.org/dwc/terms/higherClassification"/>
    <field index="12" term="http://rs.tdwg.org/dwc/terms/kingdom"/>
    <field index="13" term="http://rs.tdwg.org/dwc/terms/phylum"/>
    <field index="14" term="http://rs.tdwg.org/dwc/terms/class"/>
    <field index="15" term="http://rs.tdwg.org/dwc/terms/order"/>
    <field index="16" term="http://rs.tdwg.org/dwc/terms/superfamily"/>
    <field index="17" term="http://rs.tdwg.org/dwc/terms/family"/>
    <field index="18" term="http://rs.tdwg.org/dwc/terms/subfamily"/>
    <field index="19" term="http://rs.tdwg.org/dwc/terms/tribe"/>
    <field index="20" term="http://rs.tdwg.org/dwc/terms/subtribe"/>
    <field index="21" term="http://rs.tdwg.org/dwc/terms/genus"/>
    <field index="22" term="http://rs.tdwg.org/dwc/terms/subgenus"/>
    <field index="23" term="http://rs.tdwg.org/dwc/terms/infragenericEpithet"/>
    <field index="24" term="http://rs.tdwg.org/dwc/terms/specificEpithet"/>
    <field index="25" term="http://rs.tdwg.org/dwc/terms/infraspecificEpithet"/>
    <field index="26" term="http://rs.tdwg.org/dwc/terms/identificationVerificationStatus"/>
    <field index="27" term="https://www.inaturalist.org/terminology/identification_current"/>
  </extension>"#);
    }

    extensions
}

// TODO: allow user to specify options like org name, contact info, license, etc
/// Generates an EML (Ecological Metadata Language) file for the archive
pub fn generate_eml(metadata: &Metadata) -> String {
    let now = Utc::now().format("%Y-%m-%d");
    let package_id = format!("darwincore-archive-{}", Utc::now().format("%Y%m%d%H%M%S"));

    // Generate abstract paragraphs from metadata lines
    let abstract_paras = if metadata.abstract_lines.is_empty() {
        "\n      <para>Observations exported from iNaturalist</para>".to_string()
    } else {
        metadata.abstract_lines
            .iter()
            .map(|line| format!("\n      <para>{}</para>", line))
            .collect::<Vec<_>>()
            .join("")
    };

    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="eml://ecoinformatics.org/eml-2.1.1 http://rs.gbif.org/schema/eml-gbif-profile/1.1/eml.xsd"
         packageId="{package_id}" system="http://gbif.org" scope="system">
  <dataset>
    <title>An Example DarwinCore Archive</title>
    <creator>
      <individualName><surName>Some Organization</surName></individualName>
      <electronicMailAddress>help@some.org</electronicMailAddress>
      <onlineUrl>https://www.some.org</onlineUrl>
    </creator>
    <metadataProvider>
      <individualName><surName>Some Organization</surName></individualName>
    </metadataProvider>
    <pubDate>{now}</pubDate>
    <language>en</language>
    <abstract>{abstract_paras}
    </abstract>
    <intellectualRights>
      <para>
        This work is licensed under a
        <ulink url="https://creativecommons.org/licenses/by/4.0">
          <citetitle>Creative Commons Attribution 4.0 International License.</citetitle>
        </ulink>
      </para>
    </intellectualRights>
    <contact>
      <individualName><surName>Some Organization</surName></individualName>
      <electronicMailAddress>help@some.org</electronicMailAddress>
      <onlineUrl>https://www.some.org</onlineUrl>
    </contact>
  </dataset>
</eml:eml>"#, abstract_paras = abstract_paras)
}
