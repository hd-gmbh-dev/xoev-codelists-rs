//! Generate the codelist declarations of the XWasser model (`codes.rs`).
//!
//! The model has to state, per code type, which codelist it references and how
//! the version is determined:
//!
//! * the XML schema fixes it (code type 1/2) → `#[xoev_xwasser_code(urn, "3")]`
//! * the sender chooses it (code type 3) → `#[xoev_xwasser_code(urn, current = "5")]`,
//!   where `current` is the version published today (from the codelist data).
//!
//! Doing this by hand is how `untersuchungsverfahren` ended up declaring version
//! 2 while the data was at 3, so the numbers are generated from the schema plus
//! the dataset and never typed by hand.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use quick_xml::Reader;
use quick_xml::events::Event;

/// `urn → version fixed by the schema` (absent = sender chooses the version).
pub type SchemaVersions = BTreeMap<String, Option<String>>;

pub fn schema_versions(xsd: &std::path::Path) -> Result<SchemaVersions> {
    let source = std::fs::read_to_string(xsd).with_context(|| format!("read {}", xsd.display()))?;
    let mut reader = Reader::from_str(&source);
    let mut versions = BTreeMap::new();
    let mut urn: Option<String> = None;
    let mut fixed: Option<String> = None;
    let mut has_version_attribute = false;
    let mut in_kennung = false;

    loop {
        match reader.read_event().context("parse schema")? {
            Event::Start(start) => match local_name(start.name().as_ref()) {
                "complexType" => {
                    urn = None;
                    fixed = None;
                    has_version_attribute = false;
                }
                "attribute" => read_version_attribute(
                    start.attributes(),
                    &mut has_version_attribute,
                    &mut fixed,
                ),
                "kennung" => in_kennung = true,
                _ => {}
            },
            Event::Empty(empty) => {
                if local_name(empty.name().as_ref()) == "attribute" {
                    read_version_attribute(
                        empty.attributes(),
                        &mut has_version_attribute,
                        &mut fixed,
                    );
                }
            }
            Event::Text(text) if in_kennung => {
                urn = Some(String::from_utf8_lossy(text.as_ref()).to_string());
            }
            Event::End(end) => match local_name(end.name().as_ref()) {
                "kennung" => in_kennung = false,
                "complexType" => {
                    if let Some(urn) = urn.take()
                        && has_version_attribute
                    {
                        versions.insert(urn, fixed.take());
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(versions)
}

fn local_name(name: &[u8]) -> &str {
    let name = std::str::from_utf8(name).unwrap_or_default();
    name.rsplit(':').next().unwrap_or(name)
}

fn read_version_attribute<'a>(
    attributes: quick_xml::events::attributes::Attributes<'a>,
    has_version_attribute: &mut bool,
    fixed: &mut Option<String>,
) {
    let mut is_version = false;
    let mut fixed_value = None;
    for attribute in attributes.flatten() {
        match attribute.key.as_ref() {
            b"name" if attribute.value.as_ref() == b"listVersionID" => is_version = true,
            b"fixed" => fixed_value = Some(String::from_utf8_lossy(&attribute.value).to_string()),
            _ => {}
        }
    }
    if is_version {
        *has_version_attribute = true;
        *fixed = fixed_value;
    }
}

/// Rewrite every `#[xoev_xwasser_code(...)]` attribute of `codes_rs` in place.
///
/// Attributes may span several lines, so the rewrite works on spans, not lines.
/// Returns the sender-chosen codelists that have no declaration at all.
pub fn rewrite_codes_rs(
    codes_rs: &std::path::Path,
    schema: &SchemaVersions,
    current: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    let source = std::fs::read_to_string(codes_rs)
        .with_context(|| format!("read {}", codes_rs.display()))?;

    let marker = "#[xoev_xwasser_code(";
    let mut out = String::with_capacity(source.len());
    let mut rest = source.as_str();
    let mut seen: Vec<String> = Vec::new();
    let mut converted = 0usize;

    while let Some(start) = rest.find(marker) {
        let (before, after_start) = rest.split_at(start);
        out.push_str(before);
        let body = &after_start[marker.len()..];
        let Some(end) = body.find(")]") else {
            out.push_str(after_start);
            rest = "";
            break;
        };
        let attribute_body = &body[..end];
        let after_end = &body[end + 2..];

        let Some(urn) = attribute_body.split('"').nth(1).map(str::to_string) else {
            out.push_str(marker);
            out.push_str(attribute_body);
            out.push_str(")]");
            rest = after_end;
            continue;
        };
        seen.push(urn.clone());
        let validate = attribute_body.contains("validate") || attribute_body.contains("true");

        let rewritten = match (schema.get(&urn), current.get(&urn)) {
            (Some(Some(fixed)), _) if validate => Some(format!(
                "#[xoev_xwasser_code(\"{urn}\", fixed = \"{fixed}\", validate)]"
            )),
            (Some(Some(fixed)), _) => Some(format!(
                "#[xoev_xwasser_code(\"{urn}\", fixed = \"{fixed}\")]"
            )),
            (Some(None), Some(version)) if validate => Some(format!(
                "#[xoev_xwasser_code(\"{urn}\", current = \"{version}\", validate)]"
            )),
            (Some(None), Some(version)) => Some(format!(
                "#[xoev_xwasser_code(\"{urn}\", current = \"{version}\")]"
            )),
            // Codelists of other standards (or not covered by our data): keep
            // the semantics of the existing declaration, just name the argument.
            _ => positional_to_named(&urn, attribute_body),
        };

        if rewritten.is_some() {
            converted += 1;
        }
        match rewritten {
            Some(attribute) => out.push_str(&attribute),
            None => {
                out.push_str(marker);
                out.push_str(attribute_body);
                out.push_str(")]");
            }
        }
        rest = after_end;
    }
    out.push_str(rest);

    let mut missing = Vec::new();
    for (urn, fixed) in schema {
        if fixed.is_none() && !seen.contains(urn) {
            missing.push(urn.clone());
        }
    }

    eprintln!(
        "rewrote {converted} declaration(s) of {seen_count}",
        seen_count = seen.len()
    );
    std::fs::write(codes_rs, out).with_context(|| format!("write {}", codes_rs.display()))?;
    Ok(missing)
}

/// `(urn, "3", true)` → `(urn, fixed = "3", validate)`, `(urn, true)` → `(urn, validate)`.
fn positional_to_named(urn: &str, attribute_body: &str) -> Option<String> {
    let arguments: Vec<&str> = attribute_body
        .split(',')
        .skip(1)
        .map(str::trim)
        .filter(|argument| !argument.is_empty())
        .collect();
    let validate = arguments.contains(&"true");
    let version = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
    });

    if version.is_none() && !validate {
        return None; // nothing to name
    }
    let mut named = format!("#[xoev_xwasser_code(\"{urn}\"");
    if let Some(version) = version {
        named.push_str(&format!(", fixed = \"{version}\""));
    }
    if validate {
        named.push_str(", validate");
    }
    named.push_str(")]");
    Some(named)
}
