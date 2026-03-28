//! Parse Office Open XML chart elements into Lattice chart types.
//!
//! This module handles the `c:` namespace XML used for chart definitions
//! inside `.xlsx` files (`xl/charts/chart*.xml`).  It converts elements
//! like `c:pieChart`, `c:barChart`, etc. into our `ChartData` + `ChartOptions`
//! representation.

use lattice_charts::{ChartData, ChartOptions, ChartType, DataSeries};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;

/// A chart imported from an xlsx file.
#[derive(Debug, Clone)]
pub struct ImportedChart {
    /// The chart type as our enum.
    pub chart_type: ChartType,
    /// Chart title (from `c:title`), if present.
    pub title: Option<String>,
    /// Parsed chart data (labels + series).
    pub data: ChartData,
    /// Rendering options derived from the chart XML.
    pub options: ChartOptions,
    /// Which sheet this chart belongs to.
    pub sheet_name: String,
}

/// Parse a single chart XML string into an `ImportedChart`.
///
/// Returns `None` if no recognizable chart element is found.
/// Handles pie, bar, line, scatter, area, radar, and bubble chart types
/// including their 3D variants.
pub fn parse_chart_xml(xml: &str, sheet_name: &str) -> Option<ImportedChart> {
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();

    let mut chart_type: Option<ChartType> = None;
    let mut title: Option<String> = None;
    let mut labels: Vec<String> = Vec::new();
    let mut series_list: Vec<ParsedSeries> = Vec::new();
    let mut show_data_labels = false;

    // Parsing state
    let mut depth_stack: Vec<String> = Vec::new();
    let mut current_series: Option<ParsedSeries> = None;
    let mut in_cat = false;
    let mut in_val = false;
    let mut in_str_cache = false;
    let mut in_num_cache = false;
    let mut in_pt = false;
    let mut current_pt_idx: Option<usize> = None;
    let mut in_v = false;
    let mut in_title = false;
    let mut in_title_rich = false;
    let mut in_title_run = false;
    let mut in_title_text = false;
    let mut in_dlbls = false;
    let mut in_ser_name = false;
    #[allow(unused_assignments)]
    let mut in_ser_name_v = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = local_name(e.name().as_ref());
                depth_stack.push(local.clone());

                match local.as_str() {
                    "pieChart" | "pie3DChart" => chart_type = Some(ChartType::Pie),
                    "barChart" | "bar3DChart" => chart_type = Some(ChartType::Bar),
                    "lineChart" | "line3DChart" => chart_type = Some(ChartType::Line),
                    "scatterChart" => chart_type = Some(ChartType::Scatter),
                    "areaChart" | "area3DChart" => chart_type = Some(ChartType::Area),
                    "radarChart" => chart_type = Some(ChartType::Radar),
                    "bubbleChart" => chart_type = Some(ChartType::Bubble),
                    "ser" => {
                        current_series = Some(ParsedSeries::default());
                    }
                    "cat" if current_series.is_some() => in_cat = true,
                    "val" | "yVal" if current_series.is_some() => in_val = true,
                    "strCache" | "strRef" if in_cat => in_str_cache = true,
                    "numCache" | "numRef" if in_val => in_num_cache = true,
                    "pt" if in_str_cache || in_num_cache => {
                        in_pt = true;
                        current_pt_idx = None;
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"idx" {
                                current_pt_idx =
                                    String::from_utf8_lossy(&attr.value).parse::<usize>().ok();
                            }
                        }
                    }
                    "v" if in_pt => in_v = true,
                    "title" if depth_stack.len() <= 3 => in_title = true,
                    "rich" if in_title => in_title_rich = true,
                    "r" if in_title_rich => in_title_run = true,
                    "t" if in_title_run => in_title_text = true,
                    "dLbls" if current_series.is_some() => in_dlbls = true,
                    "tx" if current_series.is_some() && !in_title => in_ser_name = true,
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let local = local_name(e.name().as_ref());
                if in_dlbls {
                    match local.as_str() {
                        "showPercent" | "showVal" | "showCatName" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"val" {
                                    let v = String::from_utf8_lossy(&attr.value);
                                    if v == "1" || v == "true" {
                                        show_data_labels = true;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_v && in_pt {
                    if in_str_cache {
                        if let Some(idx) = current_pt_idx {
                            if idx >= labels.len() {
                                labels.resize(idx + 1, String::new());
                            }
                            labels[idx] = text;
                        }
                    } else if in_num_cache
                        && let Some(ref mut series) = current_series
                        && let Ok(val) = text.parse::<f64>()
                        && let Some(idx) = current_pt_idx
                    {
                        if idx >= series.values.len() {
                            series.values.resize(idx + 1, 0.0);
                        }
                        series.values[idx] = val;
                    }
                } else if in_title_text {
                    title = Some(text);
                } else if in_ser_name_v && let Some(ref mut series) = current_series {
                    series.name = text;
                }
            }
            Ok(Event::End(e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "ser" => {
                        if let Some(series) = current_series.take() {
                            series_list.push(series);
                        }
                        in_cat = false;
                        in_val = false;
                    }
                    "cat" => {
                        in_cat = false;
                        in_str_cache = false;
                    }
                    "val" | "yVal" => {
                        in_val = false;
                        in_num_cache = false;
                    }
                    "strCache" | "strRef" => in_str_cache = false,
                    "numCache" | "numRef" => in_num_cache = false,
                    "pt" => {
                        in_pt = false;
                        in_v = false;
                        current_pt_idx = None;
                    }
                    "v" => in_v = false,
                    "title" => {
                        in_title = false;
                        in_title_rich = false;
                        in_title_run = false;
                        in_title_text = false;
                    }
                    "rich" => in_title_rich = false,
                    "r" if in_title_rich => in_title_run = false,
                    "t" if in_title_run => in_title_text = false,
                    "dLbls" => in_dlbls = false,
                    "tx" if in_ser_name => {
                        in_ser_name = false;
                        in_ser_name_v = false;
                    }
                    _ => {}
                }
                depth_stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    let chart_type = chart_type?;

    let data = ChartData {
        labels,
        series: series_list
            .into_iter()
            .map(|s| DataSeries {
                name: s.name,
                values: s.values,
                color: None,
            })
            .collect(),
    };

    let options = ChartOptions {
        title: title.clone(),
        show_data_labels,
        ..ChartOptions::default()
    };

    Some(ImportedChart {
        chart_type,
        title,
        data,
        options,
        sheet_name: sheet_name.to_string(),
    })
}

/// Extract relationship targets whose type URL contains `type_fragment`
/// from an OPC `.rels` XML string.
pub fn extract_relationship_targets(xml: &str, type_fragment: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) if e.name().as_ref() == b"Relationship" => {
                let mut rel_type = String::new();
                let mut target = String::new();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"Type" => rel_type = String::from_utf8_lossy(&attr.value).to_string(),
                        b"Target" => target = String::from_utf8_lossy(&attr.value).to_string(),
                        _ => {}
                    }
                }
                if rel_type.to_lowercase().contains(type_fragment) && !target.is_empty() {
                    targets.push(target);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    targets
}

/// Resolve a relative path like `"../charts/chart1.xml"` against a base
/// directory like `"xl/drawings"` to produce `"xl/charts/chart1.xml"`.
pub fn resolve_relative_path(base_dir: &str, relative: &str) -> String {
    let mut parts: Vec<&str> = base_dir.split('/').collect();
    for segment in relative.split('/') {
        match segment {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Intermediate parsed series before conversion to `DataSeries`.
#[derive(Debug, Default)]
struct ParsedSeries {
    name: String,
    values: Vec<f64>,
}

/// Strip namespace prefix from an XML tag name.
/// e.g. `c:pieChart` -> `pieChart`, `a:t` -> `t`.
fn local_name(full: &[u8]) -> String {
    let s = String::from_utf8_lossy(full);
    match s.rfind(':') {
        Some(pos) => s[pos + 1..].to_string(),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_name_strips_prefix() {
        assert_eq!(local_name(b"c:pieChart"), "pieChart");
        assert_eq!(local_name(b"a:t"), "t");
        assert_eq!(local_name(b"Relationship"), "Relationship");
    }

    #[test]
    fn test_resolve_relative_path() {
        assert_eq!(
            resolve_relative_path("xl/drawings", "../charts/chart1.xml"),
            "xl/charts/chart1.xml"
        );
        assert_eq!(
            resolve_relative_path("xl/worksheets", "../drawings/drawing1.xml"),
            "xl/drawings/drawing1.xml"
        );
    }

    #[test]
    fn test_parse_pie_chart_with_title_and_data_labels() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:title><c:tx><c:rich><a:p><a:r>
      <a:t>Budget Allocation</a:t>
    </a:r></a:p></c:rich></c:tx></c:title>
    <c:plotArea>
      <c:pieChart>
        <c:ser>
          <c:idx val="0"/>
          <c:cat><c:strRef><c:strCache>
            <c:pt idx="0"><c:v>Energy</c:v></c:pt>
            <c:pt idx="1"><c:v>IT</c:v></c:pt>
            <c:pt idx="2"><c:v>HR</c:v></c:pt>
          </c:strCache></c:strRef></c:cat>
          <c:val><c:numRef><c:numCache>
            <c:pt idx="0"><c:v>150000</c:v></c:pt>
            <c:pt idx="1"><c:v>200000</c:v></c:pt>
            <c:pt idx="2"><c:v>100000</c:v></c:pt>
          </c:numCache></c:numRef></c:val>
          <c:dLbls>
            <c:showPercent val="1"/>
            <c:showCatName val="1"/>
          </c:dLbls>
        </c:ser>
      </c:pieChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;

        let chart = parse_chart_xml(xml, "Sheet1").expect("should parse pie chart");
        assert_eq!(chart.chart_type, ChartType::Pie);
        assert_eq!(chart.title, Some("Budget Allocation".to_string()));
        assert_eq!(chart.data.labels, vec!["Energy", "IT", "HR"]);
        assert_eq!(chart.data.series.len(), 1);
        assert_eq!(
            chart.data.series[0].values,
            vec![150000.0, 200000.0, 100000.0]
        );
        assert!(chart.options.show_data_labels);
        assert_eq!(chart.sheet_name, "Sheet1");
    }

    #[test]
    fn test_parse_bar_chart_no_title() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea><c:barChart>
    <c:ser>
      <c:cat><c:strRef><c:strCache>
        <c:pt idx="0"><c:v>Q1</c:v></c:pt>
        <c:pt idx="1"><c:v>Q2</c:v></c:pt>
      </c:strCache></c:strRef></c:cat>
      <c:val><c:numRef><c:numCache>
        <c:pt idx="0"><c:v>100</c:v></c:pt>
        <c:pt idx="1"><c:v>200</c:v></c:pt>
      </c:numCache></c:numRef></c:val>
    </c:ser>
  </c:barChart></c:plotArea></c:chart>
</c:chartSpace>"#;

        let chart = parse_chart_xml(xml, "Revenue").expect("should parse bar chart");
        assert_eq!(chart.chart_type, ChartType::Bar);
        assert_eq!(chart.title, None);
        assert_eq!(chart.data.labels, vec!["Q1", "Q2"]);
        assert_eq!(chart.data.series[0].values, vec![100.0, 200.0]);
        assert!(!chart.options.show_data_labels);
    }

    #[test]
    fn test_parse_line_chart() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea><c:lineChart>
    <c:ser>
      <c:val><c:numRef><c:numCache>
        <c:pt idx="0"><c:v>10</c:v></c:pt>
        <c:pt idx="1"><c:v>20</c:v></c:pt>
        <c:pt idx="2"><c:v>30</c:v></c:pt>
      </c:numCache></c:numRef></c:val>
    </c:ser>
  </c:lineChart></c:plotArea></c:chart>
</c:chartSpace>"#;

        let chart = parse_chart_xml(xml, "Data").expect("should parse line chart");
        assert_eq!(chart.chart_type, ChartType::Line);
        assert_eq!(chart.data.series[0].values, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_parse_scatter_chart_yval() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea><c:scatterChart>
    <c:ser>
      <c:yVal><c:numRef><c:numCache>
        <c:pt idx="0"><c:v>5</c:v></c:pt>
      </c:numCache></c:numRef></c:yVal>
    </c:ser>
  </c:scatterChart></c:plotArea></c:chart>
</c:chartSpace>"#;

        let chart = parse_chart_xml(xml, "XY").expect("should parse scatter chart");
        assert_eq!(chart.chart_type, ChartType::Scatter);
        assert_eq!(chart.data.series[0].values, vec![5.0]);
    }

    #[test]
    fn test_parse_area_chart() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea><c:areaChart>
    <c:ser>
      <c:val><c:numRef><c:numCache>
        <c:pt idx="0"><c:v>1</c:v></c:pt>
        <c:pt idx="1"><c:v>2</c:v></c:pt>
      </c:numCache></c:numRef></c:val>
    </c:ser>
  </c:areaChart></c:plotArea></c:chart>
</c:chartSpace>"#;

        let chart = parse_chart_xml(xml, "Sheet1").expect("should parse area chart");
        assert_eq!(chart.chart_type, ChartType::Area);
        assert_eq!(chart.data.series[0].values, vec![1.0, 2.0]);
    }

    #[test]
    fn test_parse_unknown_chart_returns_none() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea/></c:chart>
</c:chartSpace>"#;
        assert!(parse_chart_xml(xml, "Sheet1").is_none());
    }

    #[test]
    fn test_parse_multiple_series() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea><c:barChart>
    <c:ser>
      <c:cat><c:strRef><c:strCache>
        <c:pt idx="0"><c:v>Jan</c:v></c:pt>
        <c:pt idx="1"><c:v>Feb</c:v></c:pt>
      </c:strCache></c:strRef></c:cat>
      <c:val><c:numRef><c:numCache>
        <c:pt idx="0"><c:v>10</c:v></c:pt>
        <c:pt idx="1"><c:v>20</c:v></c:pt>
      </c:numCache></c:numRef></c:val>
    </c:ser>
    <c:ser>
      <c:val><c:numRef><c:numCache>
        <c:pt idx="0"><c:v>30</c:v></c:pt>
        <c:pt idx="1"><c:v>40</c:v></c:pt>
      </c:numCache></c:numRef></c:val>
    </c:ser>
  </c:barChart></c:plotArea></c:chart>
</c:chartSpace>"#;

        let chart = parse_chart_xml(xml, "Sheet1").expect("should parse");
        assert_eq!(chart.data.series.len(), 2);
        assert_eq!(chart.data.series[0].values, vec![10.0, 20.0]);
        assert_eq!(chart.data.series[1].values, vec![30.0, 40.0]);
        assert_eq!(chart.data.labels, vec!["Jan", "Feb"]);
    }

    #[test]
    fn test_extract_relationship_targets_chart() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart"
    Target="../charts/chart1.xml"/>
  <Relationship Id="rId2"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
    Target="../media/image1.png"/>
</Relationships>"#;

        let targets = extract_relationship_targets(xml, "chart");
        assert_eq!(targets, vec!["../charts/chart1.xml"]);
    }

    #[test]
    fn test_extract_relationship_targets_drawing() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing"
    Target="../drawings/drawing1.xml"/>
</Relationships>"#;

        let targets = extract_relationship_targets(xml, "drawing");
        assert_eq!(targets, vec!["../drawings/drawing1.xml"]);
    }
}
