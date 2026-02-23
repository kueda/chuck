use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use roxmltree;

use crate::dwca::{parse_delimiter, parse_meta_xml, Archive};
use crate::error::{ChuckError, Result};
use crate::search_params::SearchParams;

// ─── EML transform state ────────────────────────────────────────────────────

#[derive(Default)]
struct EmlState {
    inside_abstract: bool,
    abstract_seen: bool,
    inside_geo_coverage: bool,
    geo_coverage_seen: bool,
    inside_bounding_coords: bool,
    current_coord: Option<CoordDir>,
    inside_taxonomic_coverage: bool,
    taxonomic_coverage_seen: bool,
    dataset_depth: u32,
}

#[derive(Clone, Debug)]
enum CoordDir {
    West,
    East,
    North,
    South,
}

// ─── EML injection helpers ───────────────────────────────────────────────────

fn write_para(w: &mut Writer<Vec<u8>>, content: &str) {
    let _ = w.write_event(Event::Start(BytesStart::new("para")));
    let _ = w.write_event(Event::Text(BytesText::new(content)));
    let _ = w.write_event(Event::End(BytesEnd::new("para")));
}

fn write_abstract_block(w: &mut Writer<Vec<u8>>, desc: &str) {
    let _ = w.write_event(Event::Start(BytesStart::new("abstract")));
    write_para(w, desc);
    let _ = w.write_event(Event::End(BytesEnd::new("abstract")));
}

fn write_geo_block(w: &mut Writer<Vec<u8>>, params: &SearchParams) {
    let (nelat, nelng, swlat, swlng) = match (
        &params.nelat, &params.nelng, &params.swlat, &params.swlng,
    ) {
        (Some(a), Some(b), Some(c), Some(d)) => (a.as_str(), b.as_str(), c.as_str(), d.as_str()),
        _ => return,
    };
    let geo_desc = format!("bounding box N={nelat}/S={swlat}/E={nelng}/W={swlng}");
    let _ = w.write_event(Event::Start(BytesStart::new("geographicCoverage")));
    let _ = w.write_event(Event::Start(BytesStart::new("geographicDescription")));
    let _ = w.write_event(Event::Text(BytesText::new(&geo_desc)));
    let _ = w.write_event(Event::End(BytesEnd::new("geographicDescription")));
    let _ = w.write_event(Event::Start(BytesStart::new("boundingCoordinates")));
    for (tag, val) in [
        ("westBoundingCoordinate", swlng),
        ("eastBoundingCoordinate", nelng),
        ("northBoundingCoordinate", nelat),
        ("southBoundingCoordinate", swlat),
    ] {
        let _ = w.write_event(Event::Start(BytesStart::new(tag)));
        let _ = w.write_event(Event::Text(BytesText::new(val)));
        let _ = w.write_event(Event::End(BytesEnd::new(tag)));
    }
    let _ = w.write_event(Event::End(BytesEnd::new("boundingCoordinates")));
    let _ = w.write_event(Event::End(BytesEnd::new("geographicCoverage")));
}

fn tax_rank_from_key(key: &str) -> &'static str {
    match key {
        "kingdom" => "Kingdom",
        "phylum" => "Phylum",
        "class" => "Class",
        "order" => "Order",
        "family" => "Family",
        "genus" => "Genus",
        "scientificName" | "specificEpithet" => "Species",
        _ => "taxon",
    }
}

fn write_tax_classifications<'a>(
    w: &mut Writer<Vec<u8>>,
    filters: &[(&'a str, &'a str)],
) {
    for (key, value) in filters {
        let rank = tax_rank_from_key(key);
        let clean_val = value.trim_matches('%');
        let _ = w.write_event(Event::Start(BytesStart::new("taxonomicClassification")));
        let _ = w.write_event(Event::Start(BytesStart::new("taxonRankName")));
        let _ = w.write_event(Event::Text(BytesText::new(rank)));
        let _ = w.write_event(Event::End(BytesEnd::new("taxonRankName")));
        let _ = w.write_event(Event::Start(BytesStart::new("taxonRankValue")));
        let _ = w.write_event(Event::Text(BytesText::new(clean_val)));
        let _ = w.write_event(Event::End(BytesEnd::new("taxonRankValue")));
        let _ = w.write_event(Event::End(BytesEnd::new("taxonomicClassification")));
    }
}

fn write_full_tax_block<'a>(w: &mut Writer<Vec<u8>>, filters: &[(&'a str, &'a str)]) {
    let _ = w.write_event(Event::Start(BytesStart::new("taxonomicCoverage")));
    write_tax_classifications(w, filters);
    let _ = w.write_event(Event::End(BytesEnd::new("taxonomicCoverage")));
}

fn coord_value(coord: &CoordDir, params: &SearchParams) -> String {
    match coord {
        CoordDir::West => params.swlng.clone().unwrap_or_default(),
        CoordDir::East => params.nelng.clone().unwrap_or_default(),
        CoordDir::North => params.nelat.clone().unwrap_or_default(),
        CoordDir::South => params.swlat.clone().unwrap_or_default(),
    }
}

// ─── Core helpers ────────────────────────────────────────────────────────────

/// Returns a human-readable description of the applied filters.
fn build_filter_description(params: &SearchParams, count: usize) -> String {
    let mut parts = Vec::new();

    if let (Some(nelat), Some(nelng), Some(swlat), Some(swlng)) =
        (&params.nelat, &params.nelng, &params.swlat, &params.swlng)
    {
        parts.push(format!(
            "bounding box (N={nelat}/S={swlat}/E={nelng}/W={swlng})"
        ));
    }

    // Sort filter keys for deterministic output
    let mut sorted: Vec<(&String, &String)> = params.filters.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());
    for (key, value) in sorted {
        let clean = value.trim_matches('%');
        if !clean.is_empty() {
            parts.push(format!("{key}={clean}"));
        }
    }

    if parts.is_empty() {
        format!("Exported {count} occurrences.")
    } else {
        format!("Exported {count} filtered occurrences. Filters: {}", parts.join(", "))
    }
}

/// Filters a source CSV/TSV to only rows whose `id_column` value is in `ids`.
/// The header row is always included. Returns the filtered bytes.
fn filter_csv(
    source: &Path,
    delimiter: char,
    id_column: &str,
    ids: &HashSet<String>,
) -> Result<Vec<u8>> {
    let raw = std::fs::read_to_string(source).map_err(|e| ChuckError::FileRead {
        path: source.to_path_buf(),
        source: e,
    })?;
    // Strip UTF-8 BOM (\u{FEFF}) — common in GBIF and iNat archives
    let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);

    let mut lines = content.lines();

    let header_line = match lines.next() {
        Some(l) => l,
        None => return Err(ChuckError::CsvColumnNotFound(id_column.to_string())),
    };

    let headers = parse_csv_row(header_line, delimiter);
    let col_idx = headers
        .iter()
        .position(|h| h == id_column)
        .ok_or_else(|| ChuckError::CsvColumnNotFound(id_column.to_string()))?;

    let mut output = Vec::new();
    output.extend_from_slice(header_line.as_bytes());
    output.push(b'\n');

    for line in lines {
        if line.is_empty() {
            continue;
        }
        let fields = parse_csv_row(line, delimiter);
        if let Some(val) = fields.get(col_idx) {
            if ids.contains(val.as_str()) {
                output.extend_from_slice(line.as_bytes());
                output.push(b'\n');
            }
        }
    }

    Ok(output)
}

/// Simple delimited-row parser handling double-quoted fields and escaped quotes.
fn parse_csv_row(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => in_quotes = true,
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            }
            c if c == delimiter && !in_quotes => {
                fields.push(current.clone());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

/// Collects relative photo paths from a (filtered) multimedia CSV/TSV.
/// Values starting with `http://` or `https://` are skipped.
fn collect_photo_paths(csv_bytes: &[u8], delimiter: char) -> Vec<String> {
    let content = match std::str::from_utf8(csv_bytes) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut lines = content.lines();
    let header_line = match lines.next() {
        Some(l) => l,
        None => return Vec::new(),
    };
    let headers = parse_csv_row(header_line, delimiter);
    let identifier_idx = headers.iter().position(|h| h == "identifier");
    let access_uri_idx = headers.iter().position(|h| h == "accessURI");

    if identifier_idx.is_none() && access_uri_idx.is_none() {
        return Vec::new();
    }

    let mut paths = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let fields = parse_csv_row(line, delimiter);
        for col_idx in [identifier_idx, access_uri_idx].iter().flatten() {
            if let Some(val) = fields.get(*col_idx) {
                if !val.is_empty()
                    && !val.starts_with("http://")
                    && !val.starts_with("https://")
                {
                    paths.push(val.clone());
                }
            }
        }
    }
    paths
}

/// Modifies an EML XML document to reflect applied filters.
/// Appends a `<para>` to `<abstract>`, updates `<boundingCoordinates>` if bbox
/// present, and adds `<taxonomicClassification>` if taxonomic filters present.
fn modify_eml(eml: &str, params: &SearchParams, count: usize) -> String {
    let filter_desc = build_filter_description(params, count);

    if eml.trim().is_empty() {
        let escaped = quick_xml::escape::escape(&filter_desc);
        return format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <eml:eml xmlns:eml=\"https://eml.ecoinformatics.org/eml-2.2.0\">\
             <dataset><abstract><para>{escaped}</para></abstract></dataset></eml:eml>"
        );
    }

    let has_bbox = params.nelat.is_some()
        && params.nelng.is_some()
        && params.swlat.is_some()
        && params.swlng.is_some();

    let tax_filter_keys = [
        "kingdom",
        "phylum",
        "class",
        "order",
        "family",
        "genus",
        "scientificName",
        "specificEpithet",
    ];
    let tax_filters: Vec<(&str, &str)> = tax_filter_keys
        .iter()
        .filter_map(|&key| params.filters.get(key).map(|v| (key, v.as_str())))
        .collect();

    let mut reader = Reader::from_str(eml);
    reader.config_mut().check_end_names = false;

    let mut writer = Writer::new(Vec::<u8>::new());
    let mut state = EmlState::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(event) => {
                let owned = event.into_owned();
                match owned {
                    Event::Start(e) => {
                        let local: String = {
                            let ln = e.local_name();
                            String::from_utf8_lossy(ln.as_ref()).into_owned()
                        };
                        match local.as_str() {
                            "abstract" => state.inside_abstract = true,
                            "geographicCoverage" => {
                                state.inside_geo_coverage = true;
                                state.geo_coverage_seen = true;
                            }
                            "boundingCoordinates" => {
                                state.inside_bounding_coords = true;
                            }
                            "westBoundingCoordinate" if state.inside_bounding_coords => {
                                state.current_coord = Some(CoordDir::West);
                            }
                            "eastBoundingCoordinate" if state.inside_bounding_coords => {
                                state.current_coord = Some(CoordDir::East);
                            }
                            "northBoundingCoordinate" if state.inside_bounding_coords => {
                                state.current_coord = Some(CoordDir::North);
                            }
                            "southBoundingCoordinate" if state.inside_bounding_coords => {
                                state.current_coord = Some(CoordDir::South);
                            }
                            "taxonomicCoverage" => {
                                state.inside_taxonomic_coverage = true;
                                state.taxonomic_coverage_seen = true;
                            }
                            "dataset" => state.dataset_depth += 1,
                            _ => {}
                        }
                        let _ = writer.write_event(Event::Start(e));
                    }
                    Event::End(e) => {
                        let local: String = {
                            let ln = e.local_name();
                            String::from_utf8_lossy(ln.as_ref()).into_owned()
                        };
                        match local.as_str() {
                            "abstract" => {
                                write_para(&mut writer, &filter_desc);
                                state.inside_abstract = false;
                                state.abstract_seen = true;
                            }
                            "geographicDescription"
                                if state.inside_geo_coverage && has_bbox =>
                            {
                                let geo_text = format!(
                                    " (bounding box N={}/S={}/E={}/W={})",
                                    params.nelat.as_deref().unwrap_or(""),
                                    params.swlat.as_deref().unwrap_or(""),
                                    params.nelng.as_deref().unwrap_or(""),
                                    params.swlng.as_deref().unwrap_or(""),
                                );
                                let _ = writer.write_event(
                                    Event::Text(BytesText::new(&geo_text))
                                );
                            }
                            "boundingCoordinates" => {
                                state.inside_bounding_coords = false;
                            }
                            "westBoundingCoordinate"
                            | "eastBoundingCoordinate"
                            | "northBoundingCoordinate"
                            | "southBoundingCoordinate" => {
                                state.current_coord = None;
                            }
                            "geographicCoverage" => {
                                state.inside_geo_coverage = false;
                            }
                            "taxonomicCoverage" => {
                                if !tax_filters.is_empty() {
                                    write_tax_classifications(&mut writer, &tax_filters);
                                }
                                state.inside_taxonomic_coverage = false;
                            }
                            "dataset" => {
                                state.dataset_depth =
                                    state.dataset_depth.saturating_sub(1);
                                if state.dataset_depth == 0 {
                                    if !state.abstract_seen {
                                        write_abstract_block(&mut writer, &filter_desc);
                                    }
                                    if !state.geo_coverage_seen && has_bbox {
                                        write_geo_block(&mut writer, params);
                                    }
                                    if !state.taxonomic_coverage_seen
                                        && !tax_filters.is_empty()
                                    {
                                        write_full_tax_block(&mut writer, &tax_filters);
                                    }
                                }
                            }
                            _ => {}
                        }
                        let _ = writer.write_event(Event::End(e));
                    }
                    Event::Text(e) => {
                        if let Some(ref coord) = state.current_coord.clone() {
                            if has_bbox {
                                let value = coord_value(coord, params);
                                let _ = writer.write_event(
                                    Event::Text(BytesText::new(&value))
                                );
                            } else {
                                let _ = writer.write_event(Event::Text(e));
                            }
                        } else {
                            let _ = writer.write_event(Event::Text(e));
                        }
                    }
                    other => {
                        let _ = writer.write_event(other);
                    }
                }
            }
            Err(_) => break,
        }
        buf.clear();
    }

    String::from_utf8(writer.into_inner()).unwrap_or_default()
}

/// Minimal extension info needed for export (covers all rowTypes, not just
/// those loaded into DuckDB).
struct ExtForExport {
    location: PathBuf,
    /// Column name of the core-ID foreign key, derived from the field declaration
    /// at the coreid index. In old-format archives a separate blank `coreid`/`id`
    /// column precedes the indexed fields, so the raw index points at that blank
    /// column. Looking up the column by name avoids that trap.
    coreid_col_name: String,
    /// Raw column index from `<coreid index="N"/>` — used as a fallback when the
    /// column name cannot be found in the CSV header.
    coreid_index: usize,
    delimiter: char,
    row_type: String,
}

/// Parses ALL extension entries from meta.xml regardless of rowType.
fn parse_all_extensions_for_export(storage_dir: &Path) -> Result<Vec<ExtForExport>> {
    let meta_path = storage_dir.join("meta.xml");
    let contents = std::fs::read_to_string(&meta_path).map_err(|e| ChuckError::FileRead {
        path: meta_path.clone(),
        source: e,
    })?;
    let doc = roxmltree::Document::parse(&contents).map_err(|e| ChuckError::XmlParse {
        path: meta_path,
        source: e,
    })?;

    Ok(doc
        .descendants()
        .filter(|n| n.has_tag_name("extension"))
        .filter_map(|ext_node| {
            let row_type = ext_node.attribute("rowType")?.to_string();
            let location_text = ext_node
                .descendants()
                .filter(|n| n.has_tag_name("location"))
                .filter_map(|n| n.text())
                .next()?;
            let location = storage_dir.join(location_text);
            let coreid_index = ext_node
                .descendants()
                .find(|n| n.has_tag_name("coreid"))
                .and_then(|n| n.attribute("index"))
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            // Resolve the column NAME by looking up the <field> whose index
            // matches the coreid index. Old-format archives have a separate
            // blank `coreid` column *before* the indexed fields, so the raw
            // index 0 would point at that blank column; the declared field at
            // the same index gives us the correct column name (e.g. occurrenceID).
            let coreid_col_name = ext_node
                .descendants()
                .filter(|n| n.has_tag_name("field"))
                .find(|field_node| {
                    field_node
                        .attribute("index")
                        .and_then(|idx| idx.parse::<usize>().ok())
                        == Some(coreid_index)
                })
                .and_then(|field_node| field_node.attribute("term"))
                .and_then(|term| {
                    term.rsplit('/').next().or_else(|| term.rsplit('#').next())
                })
                .map(|s| s.to_string())
                .unwrap_or_else(|| "coreid".to_string());
            let delimiter =
                parse_delimiter(ext_node.attribute("fieldsTerminatedBy"));
            Some(ExtForExport {
                location,
                coreid_col_name,
                coreid_index,
                delimiter,
                row_type,
            })
        })
        .collect())
}

/// Filters a file to only rows whose column at `coreid_index` is in `ids`.
fn filter_csv_by_index(
    source: &Path,
    delimiter: char,
    coreid_index: usize,
    ids: &HashSet<String>,
) -> Result<Vec<u8>> {
    let raw = std::fs::read_to_string(source).map_err(|e| ChuckError::FileRead {
        path: source.to_path_buf(),
        source: e,
    })?;
    let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
    let mut lines = content.lines();
    let header_line = match lines.next() {
        Some(l) => l,
        None => return Ok(Vec::new()),
    };
    let mut output = Vec::new();
    output.extend_from_slice(header_line.as_bytes());
    output.push(b'\n');
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let fields = parse_csv_row(line, delimiter);
        if let Some(val) = fields.get(coreid_index) {
            if ids.contains(val.as_str()) {
                output.extend_from_slice(line.as_bytes());
                output.push(b'\n');
            }
        }
    }
    Ok(output)
}

pub(super) fn export_dwca_inner(
    archives_dir: PathBuf,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    let archive = Archive::current(&archives_dir)?;

    // Get IDs of all matching occurrences
    let matching_ids = archive.query_matching_ids(search_params.clone())?;

    // Parse meta.xml for source file paths and delimiter
    let (core_files, _, core_delimiter, _) = parse_meta_xml(&archive.storage_dir)?;

    // Parse ALL extension entries (including types not loaded into DuckDB)
    let all_exts = parse_all_extensions_for_export(&archive.storage_dir)?;

    // Read and modify eml.xml
    let eml_path = archive.storage_dir.join("eml.xml");
    let raw_eml = if eml_path.exists() {
        std::fs::read_to_string(&eml_path).unwrap_or_default()
    } else {
        String::new()
    };
    let modified_eml = modify_eml(&raw_eml, &search_params, matching_ids.len());

    // Read meta.xml verbatim
    let meta_path = archive.storage_dir.join("meta.xml");
    let meta_xml = std::fs::read(&meta_path).map_err(|e| ChuckError::FileRead {
        path: meta_path.clone(),
        source: e,
    })?;

    // Filter core CSV(s) and extension CSVs; collect embedded photo paths
    let deflated_opts = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let stored_opts = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Stored);

    let dest = PathBuf::from(&path);
    let out_file = std::fs::File::create(&dest).map_err(|e| ChuckError::FileOpen {
        path: dest.clone(),
        source: e,
    })?;
    let mut zip = zip::ZipWriter::new(out_file);

    // eml.xml
    zip.start_file("eml.xml", deflated_opts)
        .map_err(ChuckError::ArchiveExtraction)?;
    zip.write_all(modified_eml.as_bytes()).map_err(|e| ChuckError::FileWrite {
        path: dest.clone(),
        source: e,
    })?;

    // meta.xml (verbatim)
    zip.start_file("meta.xml", deflated_opts)
        .map_err(ChuckError::ArchiveExtraction)?;
    zip.write_all(&meta_xml).map_err(|e| ChuckError::FileWrite {
        path: dest.clone(),
        source: e,
    })?;

    // Core CSV(s)
    for core_path in &core_files {
        let rel = core_path
            .strip_prefix(&archive.storage_dir)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| {
                core_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("occurrence.csv")
                    .to_string()
            });
        // Replace backslashes (Windows) with forward slashes for ZIP compatibility
        let rel = rel.replace('\\', "/");
        let filtered =
            if core_path.exists() {
                filter_csv(core_path, core_delimiter, &archive.core_id_column, &matching_ids)?
            } else {
                Vec::new()
            };
        zip.start_file(&rel, deflated_opts)
            .map_err(ChuckError::ArchiveExtraction)?;
        zip.write_all(&filtered).map_err(|e| ChuckError::FileWrite {
            path: dest.clone(),
            source: e,
        })?;
    }

    // All extension CSVs (every rowType) + collect photo paths from multimedia
    let mut photo_paths: Vec<String> = Vec::new();
    for ext in &all_exts {
        let rel = ext
            .location
            .strip_prefix(&archive.storage_dir)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| {
                ext.location
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("extension")
                    .to_string()
            });
        let rel = rel.replace('\\', "/");
        let filtered = if ext.location.exists() {
            // Prefer filtering by column name (handles old-format archives with
            // a separate blank coreid column). Fall back to index if the name
            // isn't present in the header.
            filter_csv(&ext.location, ext.delimiter, &ext.coreid_col_name, &matching_ids)
                .or_else(|_| filter_csv_by_index(
                    &ext.location,
                    ext.delimiter,
                    ext.coreid_index,
                    &matching_ids,
                ))?
        } else {
            Vec::new()
        };

        // Collect photo paths from multimedia/audiovisual extensions
        use chuck_core::DwcaExtension;
        if chuck_core::DwcaExtension::from_row_type(&ext.row_type)
            .map(|e| matches!(e, DwcaExtension::SimpleMultimedia | DwcaExtension::Audiovisual))
            .unwrap_or(false)
        {
            let mut photos = collect_photo_paths(&filtered, ext.delimiter);
            photo_paths.append(&mut photos);
        }

        zip.start_file(&rel, deflated_opts)
            .map_err(ChuckError::ArchiveExtraction)?;
        zip.write_all(&filtered).map_err(|e| ChuckError::FileWrite {
            path: dest.clone(),
            source: e,
        })?;
    }

    // Embedded photos from archive.zip
    let archive_zip_path = archive.storage_dir.join("archive.zip");
    if archive_zip_path.exists() && !photo_paths.is_empty() {
        if let Ok(archive_file) = std::fs::File::open(&archive_zip_path) {
            if let Ok(mut src_zip) = zip::ZipArchive::new(archive_file) {
                for photo_path in &photo_paths {
                    let normalized = photo_path.replace('\\', "/");
                    let lower = normalized.to_lowercase();
                    let opts = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                        stored_opts
                    } else {
                        deflated_opts
                    };
                    match src_zip.by_name(&normalized) {
                        Ok(mut photo_file) => {
                            if zip.start_file(&normalized, opts).is_ok() {
                                let _ = std::io::copy(&mut photo_file, &mut zip);
                            }
                        }
                        Err(zip::result::ZipError::FileNotFound) => {}
                        Err(_) => {}
                    }
                }
            }
        }
    }

    zip.finish().map_err(ChuckError::ArchiveExtraction)?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_csv(name: &str, content: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("chuck_test_filter_csv_{name}.csv"));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    // ── filter_csv ────────────────────────────────────────────────────────────

    #[test]
    fn test_filter_csv_filters_rows_by_id() {
        let csv = b"occurrenceID,name\n1,Alice\n2,Bob\n3,Charlie\n";
        let path = write_temp_csv("filters_rows", csv);
        let ids: HashSet<String> = ["1".to_string(), "3".to_string()].into();

        let result = filter_csv(&path, ',', "occurrenceID", &ids).unwrap();
        let output = String::from_utf8(result).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 3, "header + 2 matching rows");
        assert_eq!(lines[0], "occurrenceID,name");
        assert!(output.contains("1,Alice"));
        assert!(output.contains("3,Charlie"));
        assert!(!output.contains("2,Bob"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_filter_csv_handles_utf8_bom() {
        // GBIF and many other archives export CSV files with a UTF-8 BOM.
        // The BOM (\xEF\xBB\xBF) appears as \u{FEFF} at the start of the
        // first header field when read as a string, which would cause column
        // lookup to fail.
        let csv = b"\xEF\xBB\xBFoccurrenceID,name\n1,Alice\n2,Bob\n";
        let path = write_temp_csv("bom_header", csv);
        let ids: HashSet<String> = ["1".to_string()].into();

        let result = filter_csv(&path, ',', "occurrenceID", &ids);
        assert!(result.is_ok(), "should handle UTF-8 BOM: {result:?}");
        let output = String::from_utf8(result.unwrap()).unwrap();
        assert!(output.contains("1,Alice"), "should include matching row");
        assert!(!output.contains("2,Bob"), "should exclude non-matching row");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_filter_csv_handles_missing_id_column() {
        let csv = b"occurrenceID,name\n1,Alice\n";
        let path = write_temp_csv("missing_col", csv);
        let ids: HashSet<String> = ["1".to_string()].into();

        let result = filter_csv(&path, ',', "nonexistentColumn", &ids);
        assert!(result.is_err(), "should error when column not found");

        std::fs::remove_file(&path).ok();
    }

    // ── collect_photo_paths ───────────────────────────────────────────────────

    #[test]
    fn test_collect_photo_paths_skips_urls() {
        let csv =
            b"occurrenceID,identifier\n1,https://example.com/photo1.jpg\n\
              2,media/photos/photo2.jpg\n3,http://other.com/photo3.jpg\n\
              4,photos/2023/photo4.jpg\n";

        let paths = collect_photo_paths(csv, ',');
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"media/photos/photo2.jpg".to_string()));
        assert!(paths.contains(&"photos/2023/photo4.jpg".to_string()));
        assert!(!paths.iter().any(|p| p.starts_with("http")));
    }

    // ── modify_eml ────────────────────────────────────────────────────────────

    #[test]
    fn test_modify_eml_with_no_eml() {
        let params = SearchParams::default();
        let result = modify_eml("", &params, 42);
        assert!(
            result.contains("<para>"),
            "should contain <para> tag: {result}"
        );
        assert!(
            result.contains("42"),
            "should mention count: {result}"
        );
    }

    #[test]
    fn test_modify_eml_appends_abstract_para() {
        let eml = r#"<?xml version="1.0"?>
<eml:eml xmlns:eml="https://eml.ecoinformatics.org/eml-2.2.0">
  <dataset>
    <abstract>
      <para>Original description</para>
    </abstract>
  </dataset>
</eml:eml>"#;
        let mut params = SearchParams::default();
        params.filters.insert("genus".to_string(), "Quercus".to_string());
        let result = modify_eml(eml, &params, 5);

        assert!(result.contains("<abstract>"), "should keep abstract: {result}");
        // There should be the injected para with the filter description
        assert!(
            result.contains("genus=Quercus"),
            "filter desc should mention genus=Quercus: {result}"
        );
        assert!(
            result.contains("5"),
            "filter desc should mention count: {result}"
        );
        assert!(
            result.contains("<para>Original description</para>"),
            "should preserve original para: {result}"
        );
    }

    #[test]
    fn test_modify_eml_updates_bounding_coordinates() {
        let eml = r#"<?xml version="1.0"?>
<eml:eml>
  <dataset>
    <geographicCoverage>
      <boundingCoordinates>
        <westBoundingCoordinate>-180</westBoundingCoordinate>
        <eastBoundingCoordinate>180</eastBoundingCoordinate>
        <northBoundingCoordinate>90</northBoundingCoordinate>
        <southBoundingCoordinate>-90</southBoundingCoordinate>
      </boundingCoordinates>
    </geographicCoverage>
  </dataset>
</eml:eml>"#;
        let params = SearchParams {
            nelat: Some("38.5".to_string()),
            nelng: Some("-122.0".to_string()),
            swlat: Some("37.0".to_string()),
            swlng: Some("-123.0".to_string()),
            ..Default::default()
        };
        let result = modify_eml(eml, &params, 10);

        assert!(
            result.contains("-123.0"),
            "west coordinate should be swlng: {result}"
        );
        assert!(
            result.contains("-122.0"),
            "east coordinate should be nelng: {result}"
        );
        assert!(
            result.contains("38.5"),
            "north coordinate should be nelat: {result}"
        );
        assert!(
            result.contains("37.0"),
            "south coordinate should be swlat: {result}"
        );
        assert!(
            !result.contains(">-180<"),
            "old west coordinate should be replaced: {result}"
        );
        assert!(
            !result.contains(">90<"),
            "old north coordinate should be replaced: {result}"
        );
    }

    #[test]
    fn test_modify_eml_adds_taxonomic_coverage() {
        let eml = r#"<?xml version="1.0"?>
<eml:eml>
  <dataset>
    <abstract><para>desc</para></abstract>
  </dataset>
</eml:eml>"#;
        let mut params = SearchParams::default();
        params.filters.insert("genus".to_string(), "Quercus".to_string());
        let result = modify_eml(eml, &params, 3);

        assert!(
            result.contains("<taxonomicClassification>"),
            "should add taxonomicClassification: {result}"
        );
        assert!(
            result.contains("Quercus"),
            "should include genus value: {result}"
        );
        assert!(
            result.contains("Genus"),
            "should include rank name: {result}"
        );
    }

    // ── export_dwca_inner ─────────────────────────────────────────────────────

    struct ExportDwcaFixture {
        base_dir: PathBuf,
        output_path: PathBuf,
    }

    impl ExportDwcaFixture {
        fn new(test_name: &str, meta_xml: &str, occurrence_csv: &[u8]) -> Self {
            use crate::db::Database;

            let base_dir = std::env::temp_dir()
                .join(format!("chuck_test_export_dwca_{test_name}"));
            std::fs::remove_dir_all(&base_dir).ok();
            let storage_dir = base_dir.join("test_archive.zip-abc123");
            std::fs::create_dir_all(&storage_dir).unwrap();

            std::fs::write(storage_dir.join("meta.xml"), meta_xml).unwrap();
            std::fs::write(storage_dir.join("occurrence.csv"), occurrence_csv).unwrap();

            let db_path = storage_dir.join("test_archive.db");
            let db = Database::create_from_core_files(
                &[storage_dir.join("occurrence.csv")],
                &[],
                &db_path,
                "occurrenceID",
            )
            .unwrap();
            drop(db);

            let output_path = std::env::temp_dir()
                .join(format!("chuck_test_export_dwca_{test_name}_output.zip"));

            Self { base_dir, output_path }
        }

        fn run(&self, search_params: SearchParams) {
            export_dwca_inner(
                self.base_dir.clone(),
                search_params,
                self.output_path.to_string_lossy().to_string(),
            )
            .unwrap();
        }

        fn zip_entry_names(&self) -> Vec<String> {
            let file = std::fs::File::open(&self.output_path).unwrap();
            let mut zip = zip::ZipArchive::new(file).unwrap();
            (0..zip.len())
                .map(|i| zip.by_index(i).unwrap().name().to_string())
                .collect()
        }
    }

    impl Drop for ExportDwcaFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.base_dir).ok();
            std::fs::remove_file(&self.output_path).ok();
        }
    }

    #[test]
    fn test_export_dwca_includes_occurrence_csv() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files><location>occurrence.csv</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
</archive>"#;
        let occurrence_csv =
            b"occurrenceID,scientificName\nobs1,Quercus agrifolia\nobs2,Pinus ponderosa\n";

        let fixture = ExportDwcaFixture::new(
            "includes_occurrence_csv",
            meta_xml,
            occurrence_csv,
        );
        fixture.run(SearchParams::default());

        let names = fixture.zip_entry_names();
        assert!(
            names.contains(&"occurrence.csv".to_string()),
            "ZIP should contain occurrence.csv, got: {names:?}"
        );
    }

    #[test]
    fn test_export_dwca_handles_tab_separated_files() {
        use crate::db::Database;

        let test_name = "export_dwca_tab_separated";
        let base_dir =
            std::env::temp_dir().join(format!("chuck_test_export_dwca_{test_name}"));
        std::fs::remove_dir_all(&base_dir).ok();
        let storage_dir = base_dir.join("test_archive.zip-abc123");
        std::fs::create_dir_all(&storage_dir).unwrap();

        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence" fieldsTerminatedBy="\t">
    <files><location>occurrence.txt</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
</archive>"#;
        let occurrence_tsv =
            b"occurrenceID\tscientificName\nobs1\tQuercus agrifolia\nobs2\tPinus ponderosa\n";

        std::fs::write(storage_dir.join("meta.xml"), meta_xml).unwrap();
        std::fs::write(storage_dir.join("occurrence.txt"), occurrence_tsv).unwrap();

        let db_path = storage_dir.join("test_archive.db");
        let db = Database::create_from_core_files(
            &[storage_dir.join("occurrence.txt")],
            &[],
            &db_path,
            "occurrenceID",
        )
        .unwrap();
        drop(db);

        let output_path =
            std::env::temp_dir().join(format!("chuck_test_{test_name}_output.zip"));
        export_dwca_inner(
            base_dir.clone(),
            SearchParams::default(),
            output_path.to_string_lossy().to_string(),
        )
        .unwrap();

        let file = std::fs::File::open(&output_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(
            names.contains(&"occurrence.txt".to_string()),
            "ZIP should contain occurrence.txt, got: {names:?}"
        );

        let mut occ = zip.by_name("occurrence.txt").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut occ, &mut content).unwrap();
        assert!(content.contains("obs1"), "should contain occurrence rows");
        assert!(content.contains("Quercus agrifolia"), "should contain scientific name");

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_dwca_includes_all_extension_files() {
        // ResourceRelationship is NOT a supported DwcaExtension type (not loaded
        // into DuckDB), but the export must still include it filtered by core ID.
        use crate::db::Database;

        let test_name = "export_dwca_all_extensions";
        let base_dir =
            std::env::temp_dir().join(format!("chuck_test_export_dwca_{test_name}"));
        std::fs::remove_dir_all(&base_dir).ok();
        let storage_dir = base_dir.join("test_archive.zip-abc123");
        std::fs::create_dir_all(&storage_dir).unwrap();

        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence" fieldsTerminatedBy="\t">
    <files><location>occurrence.txt</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
  <extension rowType="http://rs.tdwg.org/dwc/terms/ResourceRelationship"
             fieldsTerminatedBy="\t">
    <files><location>resourcerelationship.txt</location></files>
    <coreid index="0"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/relatedResourceID"/>
    <field index="2" term="http://rs.tdwg.org/dwc/terms/relationshipOfResource"/>
  </extension>
</archive>"#;

        let occurrence_tsv =
            b"occurrenceID\tscientificName\nobs1\tQuercus agrifolia\nobs2\tPinus ponderosa\n";
        let rel_tsv =
            b"occurrenceID\trelatedResourceID\trelationshipOfResource\n\
              obs1\tspecimen1\thasSpecimen\nobs2\tspecimen2\thasSpecimen\n";

        std::fs::write(storage_dir.join("meta.xml"), meta_xml).unwrap();
        std::fs::write(storage_dir.join("occurrence.txt"), occurrence_tsv).unwrap();
        std::fs::write(storage_dir.join("resourcerelationship.txt"), rel_tsv).unwrap();

        let db_path = storage_dir.join("test_archive.db");
        let db = Database::create_from_core_files(
            &[storage_dir.join("occurrence.txt")],
            &[],
            &db_path,
            "occurrenceID",
        )
        .unwrap();
        drop(db);

        let output_path =
            std::env::temp_dir().join(format!("chuck_test_{test_name}_output.zip"));
        export_dwca_inner(
            base_dir.clone(),
            SearchParams::default(),
            output_path.to_string_lossy().to_string(),
        )
        .unwrap();

        let file = std::fs::File::open(&output_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(
            names.contains(&"resourcerelationship.txt".to_string()),
            "ZIP should contain resourcerelationship.txt, got: {names:?}"
        );

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_dwca_occurrence_csv_contains_matching_rows() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files><location>occurrence.csv</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
</archive>"#;
        let occurrence_csv =
            b"occurrenceID,scientificName\nobs1,Quercus agrifolia\nobs2,Pinus ponderosa\n";

        let mut params = SearchParams::default();
        params.filters.insert("scientificName".to_string(), "Quercus agrifolia".to_string());

        let fixture = ExportDwcaFixture::new(
            "occurrence_csv_contains_rows",
            meta_xml,
            occurrence_csv,
        );
        fixture.run(params);

        let file = std::fs::File::open(&fixture.output_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut occ = zip.by_name("occurrence.csv").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut occ, &mut content).unwrap();

        assert!(content.contains("Quercus agrifolia"), "should keep matching row");
        assert!(!content.contains("Pinus ponderosa"), "should exclude non-matching row");
    }

    #[test]
    fn test_export_dwca_identification_csv_contains_data_rows() {
        // Reproduces bug: identification.csv in export was empty (header only).
        // The occurrence IDs in identification.csv must match what DuckDB returns
        // for the core occurrence IDs.
        use crate::db::Database;

        let test_name = "export_dwca_identification_data";
        let base_dir =
            std::env::temp_dir().join(format!("chuck_test_export_dwca_{test_name}"));
        std::fs::remove_dir_all(&base_dir).ok();
        let storage_dir = base_dir.join("test_archive.zip-abc123");
        std::fs::create_dir_all(&storage_dir).unwrap();

        // Simulate a chuck-generated archive where occurrenceID is a URL
        let occ_id1 = "https://www.inaturalist.org/observations/123";
        let occ_id2 = "https://www.inaturalist.org/observations/456";

        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core encoding="UTF-8" fieldsTerminatedBy="," fieldsEnclosedBy="&quot;"
        ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files><location>occurrence.csv</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
  <extension encoding="UTF-8" fieldsTerminatedBy="," fieldsEnclosedBy="&quot;"
             ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Identification">
    <files><location>identification.csv</location></files>
    <coreid index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/identificationID"/>
    <field index="2" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </extension>
</archive>"#;

        let occurrence_csv = format!(
            "occurrenceID,scientificName\n{occ_id1},Quercus agrifolia\n{occ_id2},Pinus ponderosa\n"
        );
        let identification_csv = format!(
            "occurrenceID,identificationID,scientificName\n\
             {occ_id1},id1,Quercus agrifolia\n\
             {occ_id1},id2,Quercus robur\n\
             {occ_id2},id3,Pinus ponderosa\n"
        );

        std::fs::write(storage_dir.join("meta.xml"), meta_xml).unwrap();
        std::fs::write(storage_dir.join("occurrence.csv"), occurrence_csv.as_bytes()).unwrap();
        std::fs::write(
            storage_dir.join("identification.csv"),
            identification_csv.as_bytes(),
        ).unwrap();

        let db_path = storage_dir.join("test_archive.db");
        let db = Database::create_from_core_files(
            &[storage_dir.join("occurrence.csv")],
            &[],
            &db_path,
            "occurrenceID",
        ).unwrap();
        drop(db);

        let output_path =
            std::env::temp_dir().join(format!("chuck_test_{test_name}_output.zip"));
        export_dwca_inner(
            base_dir.clone(),
            SearchParams::default(),
            output_path.to_string_lossy().to_string(),
        ).unwrap();

        let file = std::fs::File::open(&output_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(
            names.contains(&"identification.csv".to_string()),
            "ZIP should contain identification.csv, got: {names:?}"
        );

        let mut ident = zip.by_name("identification.csv").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut ident, &mut content).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(
            lines.len() > 1,
            "identification.csv should have header + data rows, got {} lines: {content:?}",
            lines.len()
        );
        assert!(content.contains("id1"), "should contain identification id1");
        assert!(content.contains("id3"), "should contain identification id3");

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_dwca_identification_filtered_by_occurrence() {
        // When a filter is applied, identification rows must only include rows
        // for occurrences that pass the filter (not all occurrences).
        use crate::db::Database;

        let test_name = "export_dwca_identification_filtered";
        let base_dir =
            std::env::temp_dir().join(format!("chuck_test_export_dwca_{test_name}"));
        std::fs::remove_dir_all(&base_dir).ok();
        let storage_dir = base_dir.join("test_archive.zip-abc123");
        std::fs::create_dir_all(&storage_dir).unwrap();

        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core encoding="UTF-8" fieldsTerminatedBy="," fieldsEnclosedBy="&quot;"
        ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files><location>occurrence.csv</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
  <extension encoding="UTF-8" fieldsTerminatedBy="," fieldsEnclosedBy="&quot;"
             ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Identification">
    <files><location>identification.csv</location></files>
    <coreid index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/identificationID"/>
    <field index="2" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </extension>
</archive>"#;

        let occurrence_csv =
            b"occurrenceID,scientificName\nobs1,Quercus agrifolia\nobs2,Pinus ponderosa\n";
        let identification_csv =
            b"occurrenceID,identificationID,scientificName\n\
              obs1,id1,Quercus agrifolia\n\
              obs1,id2,Quercus robur\n\
              obs2,id3,Pinus ponderosa\n";

        std::fs::write(storage_dir.join("meta.xml"), meta_xml).unwrap();
        std::fs::write(storage_dir.join("occurrence.csv"), occurrence_csv).unwrap();
        std::fs::write(storage_dir.join("identification.csv"), identification_csv).unwrap();

        let db_path = storage_dir.join("test_archive.db");
        let db = Database::create_from_core_files(
            &[storage_dir.join("occurrence.csv")],
            &[],
            &db_path,
            "occurrenceID",
        ).unwrap();
        drop(db);

        // Filter to only Quercus (obs1); obs2 should be excluded
        let mut params = SearchParams::default();
        params.filters.insert("scientificName".to_string(), "Quercus agrifolia".to_string());

        let output_path =
            std::env::temp_dir().join(format!("chuck_test_{test_name}_output.zip"));
        export_dwca_inner(
            base_dir.clone(),
            params,
            output_path.to_string_lossy().to_string(),
        ).unwrap();

        let file = std::fs::File::open(&output_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();

        let mut ident = zip.by_name("identification.csv").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut ident, &mut content).unwrap();

        // obs1 has identifications id1 and id2
        assert!(
            content.contains("id1"),
            "should contain id1 (obs1 identification): {content:?}"
        );
        assert!(
            content.contains("id2"),
            "should contain id2 (obs1 identification): {content:?}"
        );
        // obs2 was filtered out, so id3 should not be present
        assert!(
            !content.contains("id3"),
            "should not contain id3 (obs2 identification was filtered): {content:?}"
        );

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_dwca_identification_old_format_separate_coreid_column() {
        // Some archives (including chuck's older output) write a separate blank
        // `coreid` column at position 0, then declare fields starting at index 0
        // pointing to occurrenceID in the second column. filter_csv_by_index was
        // reading column 0 (always empty), never matching anything in matching_ids.
        // The fix: look up the coreid column by NAME (from the field declaration)
        // rather than by raw index.
        use crate::db::Database;

        let test_name = "export_dwca_old_coreid_format";
        let base_dir =
            std::env::temp_dir().join(format!("chuck_test_export_dwca_{test_name}"));
        std::fs::remove_dir_all(&base_dir).ok();
        let storage_dir = base_dir.join("test_archive.zip-abc123");
        std::fs::create_dir_all(&storage_dir).unwrap();

        // Old-style meta.xml: <coreid index="0"/> AND <field index="0" term="...occurrenceID"/>
        // both claim index 0, but the CSV has a SEPARATE `coreid` column before the fields.
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core encoding="UTF-8" fieldsTerminatedBy="," fieldsEnclosedBy="&quot;"
        ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files><location>occurrence.csv</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
  <extension encoding="UTF-8" fieldsTerminatedBy="," fieldsEnclosedBy="&quot;"
             ignoreHeaderLines="1" rowType="http://rs.tdwg.org/dwc/terms/Identification">
    <files><location>identification.csv</location></files>
    <coreid index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/identificationID"/>
    <field index="2" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </extension>
</archive>"#;

        // Old-style occurrence CSV: blank `id` column first, then occurrenceID
        let occurrence_csv =
            b"id,occurrenceID,scientificName\n,obs1,Quercus agrifolia\n,obs2,Pinus ponderosa\n";
        // Old-style identification CSV: blank `coreid` column first, then occurrenceID
        let identification_csv =
            b"coreid,occurrenceID,identificationID,scientificName\n\
              ,obs1,id1,Quercus agrifolia\n\
              ,obs1,id2,Quercus robur\n\
              ,obs2,id3,Pinus ponderosa\n";

        std::fs::write(storage_dir.join("meta.xml"), meta_xml).unwrap();
        std::fs::write(storage_dir.join("occurrence.csv"), occurrence_csv).unwrap();
        std::fs::write(storage_dir.join("identification.csv"), identification_csv).unwrap();

        let db_path = storage_dir.join("test_archive.db");
        let db = Database::create_from_core_files(
            &[storage_dir.join("occurrence.csv")],
            &[],
            &db_path,
            "occurrenceID",
        ).unwrap();
        drop(db);

        let output_path =
            std::env::temp_dir().join(format!("chuck_test_{test_name}_output.zip"));
        export_dwca_inner(
            base_dir.clone(),
            SearchParams::default(),
            output_path.to_string_lossy().to_string(),
        ).unwrap();

        let file = std::fs::File::open(&output_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();

        let mut ident = zip.by_name("identification.csv").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut ident, &mut content).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(
            lines.len() > 1,
            "identification.csv should have header + data rows (old coreid format), \
             got {} lines: {content:?}",
            lines.len()
        );
        assert!(content.contains("id1"), "should contain identification id1");

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&output_path).ok();
    }
}
