//! Embedded cover art: ID3/APIC-style pictures in audio, and EPUB package cover images.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;

use crate::handlers::zahir_ops::ZahirFileType as FileType;

/// Reject absurdly large blobs (memory / decode time).
const MAX_COVER_BYTES: usize = 8 * 1024 * 1024;

/// Return raw image bytes (JPEG/PNG/etc.) suitable for [`image::load_from_memory`], if present.
#[must_use]
pub fn try_extract_cover(path: &Path, ft: FileType) -> Option<Vec<u8>> {
    match ft {
        FileType::Audio => audio_cover_bytes(path),
        FileType::Epub => epub_cover_bytes(path),
        _ => None,
    }
}

fn audio_cover_bytes(path: &Path) -> Option<Vec<u8>> {
    let tagged = Probe::open(path).ok()?.read().ok()?;
    let tag = tagged.primary_tag()?;
    let mut fallback: Option<&[u8]> = None;
    for pic in tag.pictures() {
        let data = pic.data();
        if data.is_empty() || data.len() > MAX_COVER_BYTES {
            continue;
        }
        if pic.pic_type() == PictureType::CoverFront {
            return Some(data.to_vec());
        }
        if fallback.is_none() {
            fallback = Some(data);
        }
    }
    let b = fallback?;
    Some(b.to_vec())
}

fn epub_cover_bytes(path: &Path) -> Option<Vec<u8>> {
    let file = File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let container = read_zip_entry_string(&mut archive, "META-INF/container.xml")?;
    let opf_rel = extract_container_rootfile_full_path(&container)?;
    let opf = read_zip_entry_string(&mut archive, &opf_rel)?;
    let href = extract_opf_cover_href(&opf)?;
    let cover_zip_path = href_relative_to_opf(&opf_rel, &href);
    read_zip_entry_bytes_bounded(&mut archive, &cover_zip_path)
}

fn read_zip_entry_string(archive: &mut zip::ZipArchive<File>, name: &str) -> Option<String> {
    let mut s = String::new();
    archive.by_name(name).ok()?.read_to_string(&mut s).ok()?;
    Some(s)
}

fn read_zip_entry_bytes_bounded(
    archive: &mut zip::ZipArchive<File>,
    name: &str,
) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    archive.by_name(name).ok()?.read_to_end(&mut v).ok()?;
    if v.is_empty() || v.len() > MAX_COVER_BYTES {
        return None;
    }
    Some(v)
}

fn extract_container_rootfile_full_path(xml: &str) -> Option<String> {
    const DOUBLE: &str = r#"full-path=""#;
    const SINGLE: &str = "full-path='";
    if let Some(i) = xml.find(DOUBLE) {
        let start = i + DOUBLE.len();
        let end = xml.get(start..)?.find('"')? + start;
        return xml.get(start..end).map(str::to_string);
    }
    if let Some(i) = xml.find(SINGLE) {
        let start = i + SINGLE.len();
        let end = xml.get(start..)?.find('\'')? + start;
        return xml.get(start..end).map(str::to_string);
    }
    None
}

fn extract_xml_attr(fragment: &str, name: &str) -> Option<String> {
    extract_xml_attr_quoted(fragment, name, '"')
        .or_else(|| extract_xml_attr_quoted(fragment, name, '\''))
}

fn extract_xml_attr_quoted(fragment: &str, name: &str, quote: char) -> Option<String> {
    let needle = format!("{name}={quote}");
    let start = fragment.find(&needle)? + needle.len();
    let end = fragment.get(start..)?.find(quote)? + start;
    fragment.get(start..end).map(str::to_string)
}

fn extract_opf_cover_href(opf: &str) -> Option<String> {
    // EPUB 3: manifest item with `properties` containing `cover-image`.
    for chunk in opf.split("<item ").skip(1) {
        let end = chunk.find("/>")?;
        let frag = chunk.get(..end)?;
        if frag.to_ascii_lowercase().contains("cover-image")
            && let Some(h) = extract_xml_attr(frag, "href")
        {
            return Some(h);
        }
    }

    // EPUB 2: `<meta name="cover" content="cover-id"/>`
    for chunk in opf.split("<meta ").skip(1) {
        let end = chunk.find('/').or_else(|| chunk.find('>'))?;
        let frag = chunk.get(..end)?;
        let f = frag.to_ascii_lowercase();
        if (f.contains("name=\"cover\"") || f.contains("name='cover'"))
            && let Some(id) = extract_xml_attr(frag, "content")
        {
            return href_for_manifest_id(opf, &id);
        }
    }

    None
}

fn href_for_manifest_id(opf: &str, id: &str) -> Option<String> {
    let id_double = format!("id=\"{id}\"");
    let id_single = format!("id='{id}'");
    for chunk in opf.split("<item ").skip(1) {
        let end = chunk.find("/>")?;
        let frag = chunk.get(..end)?;
        if frag.contains(&id_double) || frag.contains(&id_single) {
            return extract_xml_attr(frag, "href");
        }
    }
    None
}

fn href_relative_to_opf(opf_zip_path: &str, href: &str) -> String {
    let opf = Path::new(opf_zip_path);
    let parent = opf.parent().unwrap_or_else(|| Path::new(""));
    parent.join(href).to_string_lossy().replace('\\', "/")
}
