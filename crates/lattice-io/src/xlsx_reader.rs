//! Read `.xlsx` files using calamine and convert to Lattice `Workbook`.

use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};

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
                        formula: None,
                        format: Default::default(),
                        style_id: 0,
                        comment: None,
                    };
                    sheet.set_cell(row_idx as u32, col_idx as u32, cell);
                }
            }
        }
    }

    // Set active sheet to the first one.
    workbook.active_sheet = sheet_names[0].clone();

    Ok(workbook)
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
            // Use as_f64() to get the serial number.
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
}
