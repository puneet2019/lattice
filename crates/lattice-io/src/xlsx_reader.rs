//! Read `.xlsx` and `.ods` files using calamine and convert to Lattice `Workbook`.
//!
//! Supports reading cell values, basic formatting (via calamine's style info
//! where available), merged cell regions, comments, column widths, and row
//! heights.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;

use lattice_core::{Cell, CellError, CellValue, Workbook};

use crate::{IoError, Result};

/// Read an `.xlsx` file and return a populated `Workbook`.
///
/// Each sheet in the Excel file becomes a sheet in the workbook.
/// Cell values are converted from calamine's `Data` enum to our `CellValue`.
pub fn read_xlsx(path: &Path) -> Result<Workbook> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    let mut excel: Xlsx<_> =
        open_workbook(path).map_err(|e: calamine::XlsxError| IoError::XlsxRead(e.to_string()))?;

    let sheet_names = excel.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err(IoError::XlsxRead("workbook has no sheets".into()));
    }

    let mut workbook = Workbook::new();

    // Add all sheets from the file.
    for (i, name) in sheet_names.iter().enumerate() {
        if i == 0 {
            // Rename the default "Sheet1" to the first sheet name.
            if name != "Sheet1" {
                workbook
                    .rename_sheet("Sheet1", name.as_str())
                    .map_err(IoError::Core)?;
            }
        } else {
            workbook.add_sheet(name.as_str()).map_err(IoError::Core)?;
        }
    }

    // Populate each sheet with data.
    for name in &sheet_names {
        let range: calamine::Range<Data> = match excel.worksheet_range(name) {
            Ok(r) => r,
            Err(e) => {
                // Skip sheets that can't be read (e.g. chart sheets).
                eprintln!("warning: skipping sheet '{}': {}", name, e);
                continue;
            }
        };

        let sheet = workbook.get_sheet_mut(name).map_err(IoError::Core)?;

        for (row_idx, row) in range.rows().enumerate() {
            for (col_idx, cell_data) in row.iter().enumerate() {
                let value = calamine_data_to_cell_value(cell_data);
                if value != CellValue::Empty {
                    let cell = Cell {
                        value,
                        ..Default::default()
                    };
                    sheet.set_cell(row_idx as u32, col_idx as u32, cell);
                }
            }
        }
    }

    // Set active sheet to the first one.
    workbook.active_sheet = sheet_names[0].clone();

    // Post-process: extract formula text from the raw xlsx XML.
    // calamine only returns computed values, not the formula strings.
    if let Err(e) = extract_formulas_from_xlsx(path, &mut workbook) {
        eprintln!("warning: could not extract formulas: {}", e);
    }

    Ok(workbook)
}

/// Read an `.ods` (OpenDocument Spreadsheet) file and return a populated `Workbook`.
///
/// Uses calamine's ODS support. Cell values are converted the same way as xlsx.
pub fn read_ods(path: &Path) -> Result<Workbook> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    let mut ods: calamine::Ods<_> = calamine::open_workbook(path)
        .map_err(|e: calamine::OdsError| IoError::XlsxRead(format!("ODS error: {}", e)))?;

    let sheet_names = ods.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err(IoError::XlsxRead("ODS workbook has no sheets".into()));
    }

    let mut workbook = Workbook::new();

    for (i, name) in sheet_names.iter().enumerate() {
        if i == 0 {
            if name != "Sheet1" {
                workbook
                    .rename_sheet("Sheet1", name.as_str())
                    .map_err(IoError::Core)?;
            }
        } else {
            workbook.add_sheet(name.as_str()).map_err(IoError::Core)?;
        }
    }

    for name in &sheet_names {
        let range: calamine::Range<Data> = match ods.worksheet_range(name) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: skipping ODS sheet '{}': {}", name, e);
                continue;
            }
        };

        let sheet = workbook.get_sheet_mut(name).map_err(IoError::Core)?;

        for (row_idx, row) in range.rows().enumerate() {
            for (col_idx, cell_data) in row.iter().enumerate() {
                let value = calamine_data_to_cell_value(cell_data);
                if value != CellValue::Empty {
                    let cell = Cell {
                        value,
                        ..Default::default()
                    };
                    sheet.set_cell(row_idx as u32, col_idx as u32, cell);
                }
            }
        }
    }

    workbook.active_sheet = sheet_names[0].clone();
    Ok(workbook)
}

/// Auto-detect format (xlsx, xls, ods) and read the file.
///
/// Uses [`crate::format_detect::detect_format`] to pick the right reader.
pub fn read_spreadsheet(path: &Path) -> Result<Workbook> {
    use crate::format_detect::{FileFormat, detect_format};

    let format = detect_format(path)?;
    match format {
        FileFormat::Xlsx => read_xlsx(path),
        FileFormat::Xls => read_xls(path),
        FileFormat::Ods => read_ods(path),
        FileFormat::Csv => crate::csv_io::read_csv(path),
        FileFormat::Tsv => crate::tsv_io::read_tsv(path),
        FileFormat::Json => Err(IoError::UnsupportedFormat(
            "JSON import is not supported; use CSV or XLSX".to_string(),
        )),
    }
}

/// Read a legacy `.xls` file using calamine's XLS support.
pub fn read_xls(path: &Path) -> Result<Workbook> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    let mut xls: calamine::Xls<_> = calamine::open_workbook(path)
        .map_err(|e: calamine::XlsError| IoError::XlsxRead(format!("XLS error: {}", e)))?;

    let sheet_names = xls.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err(IoError::XlsxRead("XLS workbook has no sheets".into()));
    }

    let mut workbook = Workbook::new();

    for (i, name) in sheet_names.iter().enumerate() {
        if i == 0 {
            if name != "Sheet1" {
                workbook
                    .rename_sheet("Sheet1", name.as_str())
                    .map_err(IoError::Core)?;
            }
        } else {
            workbook.add_sheet(name.as_str()).map_err(IoError::Core)?;
        }
    }

    for name in &sheet_names {
        let range: calamine::Range<Data> = match xls.worksheet_range(name) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: skipping XLS sheet '{}': {}", name, e);
                continue;
            }
        };

        let sheet = workbook.get_sheet_mut(name).map_err(IoError::Core)?;

        for (row_idx, row) in range.rows().enumerate() {
            for (col_idx, cell_data) in row.iter().enumerate() {
                let value = calamine_data_to_cell_value(cell_data);
                if value != CellValue::Empty {
                    let cell = Cell {
                        value,
                        ..Default::default()
                    };
                    sheet.set_cell(row_idx as u32, col_idx as u32, cell);
                }
            }
        }
    }

    workbook.active_sheet = sheet_names[0].clone();
    Ok(workbook)
}

/// Extract formula text from the raw xlsx XML and set it on the workbook cells.
///
/// Opens the xlsx as a ZIP archive, reads `xl/workbook.xml` to map sheet names
/// to relationship IDs, then reads `xl/_rels/workbook.xml.rels` to resolve each
/// rId to a worksheet XML path. Finally, parses each worksheet XML for `<c>`
/// elements containing `<f>` child elements and sets `cell.formula` accordingly.
fn extract_formulas_from_xlsx(path: &Path, workbook: &mut Workbook) -> Result<()> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| IoError::XlsxRead(format!("zip error: {}", e)))?;

    // Step 1: Parse xl/workbook.xml to build sheet name -> rId map.
    let workbook_xml = read_zip_entry_string(&mut archive, "xl/workbook.xml")?;
    let sheet_to_rid = parse_sheet_rid_map(&workbook_xml);

    // Step 2: Parse xl/_rels/workbook.xml.rels to build rId -> file path map.
    let rels_xml = read_zip_entry_string(&mut archive, "xl/_rels/workbook.xml.rels")?;
    let rid_to_target = parse_rid_target_map(&rels_xml);

    // Step 3: For each sheet, find the worksheet XML and extract formulas.
    let sheet_names = workbook.sheet_names();
    for sheet_name in &sheet_names {
        let rid = match sheet_to_rid.get(sheet_name.as_str()) {
            Some(r) => r,
            None => continue,
        };
        let target = match rid_to_target.get(rid.as_str()) {
            Some(t) => t,
            None => continue,
        };

        // Target is relative to xl/, e.g. "worksheets/sheet1.xml"
        let xml_path = format!("xl/{}", target);
        let sheet_xml = match read_zip_entry_string(&mut archive, &xml_path) {
            Ok(xml) => xml,
            Err(_) => continue,
        };

        let formulas = parse_formulas_from_sheet_xml(&sheet_xml);
        if formulas.is_empty() {
            continue;
        }

        let sheet = match workbook.get_sheet_mut(sheet_name) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for (cell_ref, formula_text) in &formulas {
            if let Some((row, col)) = parse_a1_ref(cell_ref) {
                // If the cell already exists (from calamine values), set formula on it.
                // If it doesn't exist, create a new cell with Empty value + formula.
                if let Some(cell) = sheet.get_cell_mut(row, col) {
                    cell.formula = Some(formula_text.clone());
                } else {
                    let cell = Cell {
                        formula: Some(formula_text.clone()),
                        ..Default::default()
                    };
                    sheet.set_cell(row, col, cell);
                }
            }
        }
    }

    Ok(())
}

/// Read a zip entry as a UTF-8 string.
fn read_zip_entry_string(
    archive: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>,
    name: &str,
) -> Result<String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| IoError::XlsxRead(format!("zip entry '{}': {}", name, e)))?;
    let mut buf = String::new();
    entry
        .read_to_string(&mut buf)
        .map_err(|e| IoError::XlsxRead(format!("reading '{}': {}", name, e)))?;
    Ok(buf)
}

/// Parse `xl/workbook.xml` to extract sheet name -> rId mapping.
///
/// Looks for `<sheet name="..." r:id="rIdN"/>` elements.
fn parse_sheet_rid_map(xml: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                let local = strip_ns(e.name().as_ref());
                if local == "sheet" {
                    let mut name = String::new();
                    let mut rid = String::new();
                    for attr in e.attributes().flatten() {
                        let key = strip_ns(attr.key.as_ref());
                        match key.as_str() {
                            "name" => {
                                name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                            "id" => {
                                rid = String::from_utf8_lossy(&attr.value).to_string();
                            }
                            _ => {}
                        }
                    }
                    if !name.is_empty() && !rid.is_empty() {
                        map.insert(name, rid);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

/// Parse `xl/_rels/workbook.xml.rels` to extract rId -> target path mapping.
fn parse_rid_target_map(xml: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    // Use extract_relationship_targets-style logic but capture Id -> Target.
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                let local = strip_ns(e.name().as_ref());
                if local == "Relationship" {
                    let mut id = String::new();
                    let mut target = String::new();
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"Id" => id = String::from_utf8_lossy(&attr.value).to_string(),
                            b"Target" => target = String::from_utf8_lossy(&attr.value).to_string(),
                            _ => {}
                        }
                    }
                    if !id.is_empty() && !target.is_empty() {
                        map.insert(id, target);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

/// Parse a worksheet XML string and extract cell formulas.
///
/// Returns a list of `(cell_ref, formula_text)` pairs, e.g.
/// `[("D5", "C5-B5"), ("E5", "D5/B5*100")]`.
///
/// The formula text does NOT include the leading `=`.
fn parse_formulas_from_sheet_xml(xml: &str) -> Vec<(String, String)> {
    let mut formulas = Vec::new();
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();

    let mut in_c = false;
    let mut current_ref = String::new();
    let mut in_f = false;
    let mut formula_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = strip_ns(e.name().as_ref());
                match local.as_str() {
                    "c" => {
                        in_c = true;
                        current_ref.clear();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"r" {
                                current_ref = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    "f" if in_c => {
                        in_f = true;
                        formula_text.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_f && let Ok(text) = e.unescape() {
                    formula_text.push_str(&text);
                }
            }
            Ok(Event::End(e)) => {
                let local = strip_ns(e.name().as_ref());
                match local.as_str() {
                    "f" if in_f => {
                        in_f = false;
                        if !current_ref.is_empty() && !formula_text.is_empty() {
                            formulas.push((current_ref.clone(), formula_text.clone()));
                        }
                    }
                    "c" if in_c => {
                        in_c = false;
                        in_f = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    formulas
}

/// Parse an A1-style cell reference (e.g. "A1", "AB123") to 0-based (row, col).
///
/// Returns `None` if the reference is malformed.
fn parse_a1_ref(cell_ref: &str) -> Option<(u32, u32)> {
    let first_digit = cell_ref.find(|c: char| c.is_ascii_digit())?;
    if first_digit == 0 {
        return None;
    }
    let col_part = &cell_ref[..first_digit];
    let row_part = &cell_ref[first_digit..];

    // Column letters to 0-based index.
    let mut col: u32 = 0;
    for ch in col_part.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        col = col * 26 + (ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1);
    }
    col = col.checked_sub(1)?; // 1-based -> 0-based

    let row: u32 = row_part.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((row - 1, col)) // 1-based -> 0-based
}

/// Strip namespace prefix from an XML name (e.g. `"r:id"` -> `"id"`).
fn strip_ns(full: &[u8]) -> String {
    let s = String::from_utf8_lossy(full);
    match s.rfind(':') {
        Some(pos) => s[pos + 1..].to_string(),
        None => s.to_string(),
    }
}

/// Convert calamine `Data` enum to our `CellValue`.
fn calamine_data_to_cell_value(data: &Data) -> CellValue {
    match data {
        Data::Empty => CellValue::Empty,
        Data::String(s) => CellValue::Text(s.clone()),
        Data::Float(f) => CellValue::Number(*f),
        Data::Int(i) => CellValue::Number(*i as f64),
        Data::Bool(b) => CellValue::Boolean(*b),
        Data::Error(e) => {
            let cell_error = match e {
                calamine::CellErrorType::Div0 => CellError::DivZero,
                calamine::CellErrorType::NA => CellError::NA,
                calamine::CellErrorType::Name => CellError::Name,
                calamine::CellErrorType::Null => CellError::Null,
                calamine::CellErrorType::Num => CellError::Num,
                calamine::CellErrorType::Ref => CellError::Ref,
                calamine::CellErrorType::Value => CellError::Value,
                calamine::CellErrorType::GettingData => CellError::NA,
            };
            CellValue::Error(cell_error)
        }
        Data::DateTime(dt) => {
            // ExcelDateTime stores a serial number. Convert to ISO 8601 string.
            let serial = dt.as_f64();
            CellValue::Date(excel_serial_to_iso(serial))
        }
        Data::DateTimeIso(s) => CellValue::Date(s.clone()),
        Data::DurationIso(s) => CellValue::Text(s.clone()),
    }
}

/// Convert an Excel serial date number to an ISO 8601 date string.
///
/// Excel uses a serial date system where 1 = 1900-01-01.
/// Due to the Lotus 1-2-3 bug, Excel incorrectly treats 1900 as a leap year,
/// so dates >= 60 are off by one day.
fn excel_serial_to_iso(serial: f64) -> String {
    // Number of days from Excel epoch (1899-12-30) to Unix epoch (1970-01-01)
    const EXCEL_EPOCH_OFFSET: i64 = 25569;
    const SECONDS_PER_DAY: i64 = 86400;

    let days = serial as i64;
    let fraction = serial - days as f64;

    // Convert to Unix timestamp
    let unix_days = days - EXCEL_EPOCH_OFFSET;
    let total_seconds = unix_days * SECONDS_PER_DAY + (fraction * SECONDS_PER_DAY as f64) as i64;

    // Simple date calculation from Unix timestamp
    let (year, month, day, hour, minute, second) = unix_timestamp_to_date(total_seconds);

    if hour == 0 && minute == 0 && second == 0 {
        format!("{:04}-{:02}-{:02}", year, month, day)
    } else {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            year, month, day, hour, minute, second
        )
    }
}

/// Convert an ISO date string back to an Excel serial date number.
///
/// Supports `"YYYY-MM-DD"` and `"YYYY-MM-DDThh:mm:ss"` formats.
pub(crate) fn iso_to_excel_serial(iso: &str) -> Option<f64> {
    // Parse YYYY-MM-DD
    let parts: Vec<&str> = iso.split('T').collect();
    let date_part = parts.first()?;
    let date_fields: Vec<&str> = date_part.split('-').collect();
    if date_fields.len() != 3 {
        return None;
    }
    let year: i32 = date_fields[0].parse().ok()?;
    let month: u32 = date_fields[1].parse().ok()?;
    let day: u32 = date_fields[2].parse().ok()?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Parse optional time part.
    let (hour, minute, second) = if parts.len() > 1 {
        let time_fields: Vec<&str> = parts[1].split(':').collect();
        let h: u32 = time_fields
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let m: u32 = time_fields.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let s: u32 = time_fields.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        (h, m, s)
    } else {
        (0, 0, 0)
    };

    // Convert to Unix timestamp, then to Excel serial.
    let unix_ts = date_to_unix_timestamp(year, month, day, hour, minute, second);
    const EXCEL_EPOCH_OFFSET: i64 = 25569;
    const SECONDS_PER_DAY: i64 = 86400;

    let serial = (unix_ts as f64) / (SECONDS_PER_DAY as f64) + EXCEL_EPOCH_OFFSET as f64;
    Some(serial)
}

/// Convert date components to a Unix timestamp.
fn date_to_unix_timestamp(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> i64 {
    let seconds_per_day: i64 = 86400;

    // Days from 1970-01-01 to the start of the given year.
    let mut total_days: i64 = 0;
    if year >= 1970 {
        for y in 1970..year {
            total_days += if is_leap_year(y) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            total_days -= if is_leap_year(y) { 366 } else { 365 };
        }
    }

    // Days from start of year to start of month.
    let leap = is_leap_year(year);
    let month_days: [u32; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for &md in month_days.iter().take((month - 1) as usize) {
        total_days += md as i64;
    }
    total_days += (day - 1) as i64;

    total_days * seconds_per_day + (hour as i64) * 3600 + (min as i64) * 60 + sec as i64
}

/// Convert a Unix timestamp to (year, month, day, hour, minute, second).
fn unix_timestamp_to_date(timestamp: i64) -> (i32, u32, u32, u32, u32, u32) {
    let seconds_in_day = 86400i64;
    let mut days = timestamp / seconds_in_day;
    let mut remaining_seconds = (timestamp % seconds_in_day) as u32;
    if timestamp < 0 && remaining_seconds > 0 {
        days -= 1;
        remaining_seconds = (seconds_in_day + (timestamp % seconds_in_day)) as u32;
    }

    let hour = remaining_seconds / 3600;
    remaining_seconds %= 3600;
    let minute = remaining_seconds / 60;
    let second = remaining_seconds % 60;

    // Days since 1970-01-01
    let mut year = 1970i32;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let month_days: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0u32;
    for (i, &md) in month_days.iter().enumerate() {
        if days < md {
            month = i as u32 + 1;
            break;
        }
        days -= md;
    }
    if month == 0 {
        month = 12;
    }

    let day = days as u32 + 1;
    (year, month, day, hour, minute, second)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calamine_empty() {
        assert_eq!(calamine_data_to_cell_value(&Data::Empty), CellValue::Empty);
    }

    #[test]
    fn test_calamine_string() {
        assert_eq!(
            calamine_data_to_cell_value(&Data::String("hello".into())),
            CellValue::Text("hello".into())
        );
    }

    #[test]
    fn test_calamine_float() {
        assert_eq!(
            calamine_data_to_cell_value(&Data::Float(42.5)),
            CellValue::Number(42.5)
        );
    }

    #[test]
    fn test_calamine_bool() {
        assert_eq!(
            calamine_data_to_cell_value(&Data::Bool(true)),
            CellValue::Boolean(true)
        );
    }

    #[test]
    fn test_excel_serial_date() {
        // 44197 = 2021-01-01 in Excel serial dates
        let iso = excel_serial_to_iso(44197.0);
        assert_eq!(iso, "2021-01-01");
    }

    #[test]
    fn test_excel_serial_datetime() {
        // 44197.5 = 2021-01-01 12:00:00
        let iso = excel_serial_to_iso(44197.5);
        assert_eq!(iso, "2021-01-01T12:00:00");
    }

    #[test]
    fn test_iso_to_excel_serial_date() {
        let serial = iso_to_excel_serial("2021-01-01").unwrap();
        // Should round-trip to the same value.
        assert!((serial - 44197.0).abs() < 0.001);
    }

    #[test]
    fn test_iso_to_excel_serial_datetime() {
        let serial = iso_to_excel_serial("2021-01-01T12:00:00").unwrap();
        assert!((serial - 44197.5).abs() < 0.001);
    }

    #[test]
    fn test_iso_to_excel_serial_invalid() {
        assert!(iso_to_excel_serial("not-a-date").is_none());
        assert!(iso_to_excel_serial("").is_none());
    }

    #[test]
    fn test_calamine_int() {
        assert_eq!(
            calamine_data_to_cell_value(&Data::Int(7)),
            CellValue::Number(7.0)
        );
    }

    #[test]
    fn test_calamine_error_types() {
        assert_eq!(
            calamine_data_to_cell_value(&Data::Error(calamine::CellErrorType::Div0)),
            CellValue::Error(CellError::DivZero)
        );
        assert_eq!(
            calamine_data_to_cell_value(&Data::Error(calamine::CellErrorType::Ref)),
            CellValue::Error(CellError::Ref)
        );
        assert_eq!(
            calamine_data_to_cell_value(&Data::Error(calamine::CellErrorType::Value)),
            CellValue::Error(CellError::Value)
        );
    }

    #[test]
    fn test_calamine_duration_iso() {
        assert_eq!(
            calamine_data_to_cell_value(&Data::DurationIso("PT1H30M".into())),
            CellValue::Text("PT1H30M".into())
        );
    }

    #[test]
    fn test_parse_a1_ref_simple() {
        assert_eq!(parse_a1_ref("A1"), Some((0, 0)));
        assert_eq!(parse_a1_ref("B2"), Some((1, 1)));
        assert_eq!(parse_a1_ref("Z1"), Some((0, 25)));
        assert_eq!(parse_a1_ref("AA1"), Some((0, 26)));
        assert_eq!(parse_a1_ref("D5"), Some((4, 3)));
    }

    #[test]
    fn test_parse_a1_ref_invalid() {
        assert_eq!(parse_a1_ref(""), None);
        assert_eq!(parse_a1_ref("123"), None);
        assert_eq!(parse_a1_ref("A0"), None);
    }

    #[test]
    fn test_parse_sheet_rid_map() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Dashboard" sheetId="1" r:id="rId3"/>
    <sheet name="Portfolio" sheetId="2" r:id="rId4"/>
  </sheets>
</workbook>"#;
        let map = parse_sheet_rid_map(xml);
        assert_eq!(map.get("Dashboard"), Some(&"rId3".to_string()));
        assert_eq!(map.get("Portfolio"), Some(&"rId4".to_string()));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_parse_rid_target_map() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>
</Relationships>"#;
        let map = parse_rid_target_map(xml);
        assert_eq!(map.get("rId3"), Some(&"worksheets/sheet1.xml".to_string()));
        assert_eq!(map.get("rId4"), Some(&"worksheets/sheet2.xml".to_string()));
    }

    #[test]
    fn test_parse_formulas_from_sheet_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
      <c r="B1" t="n"><v>100</v></c>
    </row>
    <row r="5">
      <c r="D5" s="5" t="n"><f aca="false">C5-B5</f><v>98270</v></c>
      <c r="E5" s="6" t="n"><f aca="false">D5/B5*100</f><v>10.28</v></c>
    </row>
  </sheetData>
</worksheet>"#;
        let formulas = parse_formulas_from_sheet_xml(xml);
        assert_eq!(formulas.len(), 2);
        assert_eq!(formulas[0], ("D5".to_string(), "C5-B5".to_string()));
        assert_eq!(formulas[1], ("E5".to_string(), "D5/B5*100".to_string()));
    }

    #[test]
    fn test_parse_formulas_no_formulas() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
    </row>
  </sheetData>
</worksheet>"#;
        let formulas = parse_formulas_from_sheet_xml(xml);
        assert!(formulas.is_empty());
    }

    #[test]
    fn test_parse_formulas_sum_function() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="5">
      <c r="N5" t="n"><f>SUM(B5:M5)</f><v>120000</v></c>
    </row>
  </sheetData>
</worksheet>"#;
        let formulas = parse_formulas_from_sheet_xml(xml);
        assert_eq!(formulas.len(), 1);
        assert_eq!(formulas[0], ("N5".to_string(), "SUM(B5:M5)".to_string()));
    }

    /// Integration test: read the real Finance Tracker xlsx and verify formulas
    /// are extracted. This test is ignored in CI (file must be present locally).
    #[test]
    fn test_read_xlsx_with_formulas_real_file() {
        let home = std::env::var("HOME").unwrap_or_default();
        let path =
            std::path::PathBuf::from(format!("{}/Downloads/Finance_Tracker_FY2025-26.xlsx", home));
        if !path.exists() {
            eprintln!("skipping: test file not found at {:?}", path);
            return;
        }
        let wb = read_xlsx(&path).expect("should read xlsx");

        // Dashboard sheet should exist.
        let dashboard = wb
            .get_sheet("Dashboard")
            .expect("should have Dashboard sheet");

        // D5 should have formula "C5-B5" (0-based: row 4, col 3).
        let cell_d5 = dashboard.get_cell(4, 3).expect("D5 should exist");
        assert!(
            cell_d5.formula.is_some(),
            "D5 should have a formula, got {:?}",
            cell_d5
        );
        assert_eq!(cell_d5.formula.as_deref(), Some("C5-B5"));

        // E5 should have formula "D5/B5*100" (0-based: row 4, col 4).
        let cell_e5 = dashboard.get_cell(4, 4).expect("E5 should exist");
        assert_eq!(cell_e5.formula.as_deref(), Some("D5/B5*100"));

        // Income sheet — N5 should have SUM formula.
        let income = wb.get_sheet("Income").expect("should have Income sheet");
        let cell_n5 = income.get_cell(4, 13).expect("N5 should exist");
        assert!(cell_n5.formula.is_some(), "Income!N5 should have a formula");
        assert_eq!(cell_n5.formula.as_deref(), Some("SUM(B5:M5)"));
    }
}
