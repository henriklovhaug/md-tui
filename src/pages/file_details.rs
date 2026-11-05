use std::path::Path;

use chrono::{DateTime, Local};

/// Columns taken by the modified time and size.
pub(super) const DETAILS_WIDTH: usize = 26;

pub(super) fn file_details(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?.into();
    Some(format_details(modified, metadata.len()))
}

fn format_details(modified: DateTime<Local>, size: u64) -> String {
    format!(
        "{}  {:>8}",
        modified.format("%Y-%m-%d %H:%M"),
        format_size(size)
    )
}

fn format_size(bytes: u64) -> String {
    let size = bytes as f64;
    match bytes {
        0..1_000 => format!("{bytes} B"),
        1_000..1_000_000 => format!("{:.1} KB", size / 1e3),
        1_000_000..1_000_000_000 => format!("{:.1} MB", size / 1e6),
        _ => format!("{:.1} GB", size / 1e9),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    use super::*;
    use crate::pages::file_explorer::{FileTree, MdFile};

    const DETAILS: &str = "2026-11-04 14:32    1.5 KB";

    fn modified() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 4, 14, 32, 0).unwrap()
    }

    fn file(name: &str, details: Option<&str>) -> MdFile {
        MdFile {
            path: format!("./{name}"),
            name: name.to_string(),
            details: details.map(ToString::to_string),
        }
    }

    fn rows(files: Vec<MdFile>, width: u16) -> Vec<String> {
        let mut tree = FileTree::new();
        for file in files {
            tree.add_file(file);
        }
        let area = Rect::new(0, 0, width, 12);
        let mut buf = Buffer::empty(area);
        tree.render(area, &mut buf);
        (0..area.height)
            .map(|y| {
                let row = (0..area.width).map(|x| buf[(x, y)].symbol());
                row.collect::<String>().trim_end().to_string()
            })
            .collect()
    }

    #[test]
    fn sizes_use_the_largest_unit_that_fits() {
        for (bytes, expected) in [
            (0, "0 B"),
            (999, "999 B"),
            (1_000, "1.0 KB"),
            (12_345, "12.3 KB"),
            (1_500_000, "1.5 MB"),
            (2_000_000_000, "2.0 GB"),
        ] {
            assert_eq!(format_size(bytes), expected);
        }
    }

    #[test]
    fn details_fit_their_column() {
        let details = format_details(modified(), 1_500);
        assert_eq!(details, DETAILS);
        assert_eq!(details.len(), DETAILS_WIDTH);
    }

    #[test]
    fn details_come_from_the_file_metadata() {
        let path = std::env::temp_dir().join(format!("mdt-details-{}.md", std::process::id()));
        std::fs::write(&path, vec![b'x'; 1_500]).expect("write");
        std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open")
            .set_modified(modified().into())
            .expect("set modified time");

        let details = file_details(&path);
        std::fs::remove_file(&path).expect("remove");

        assert_eq!(details.as_deref(), Some(DETAILS));
        assert_eq!(file_details(&path), None);
    }

    #[test]
    fn details_line_up_on_a_page() {
        let rows = rows(
            vec![
                file("a.md", Some(DETAILS)),
                file("longer-name.md", Some(DETAILS)),
                file("日本語.md", Some(DETAILS)),
            ],
            60,
        );
        assert_eq!(rows[1], format!("  a.md            {DETAILS}"));
        assert_eq!(rows[2], "  ./a.md");
        assert_eq!(rows[4], format!("  longer-name.md  {DETAILS}"));
        let wide = rows[7].chars().collect::<Vec<_>>();
        let details = wide[18..].iter().collect::<String>();
        assert_eq!(details, DETAILS);
    }

    #[test]
    fn a_name_too_long_for_the_column_stays_whole() {
        let long = "a-name-longer-than-its-column.md";
        let rows = rows(
            vec![file("a.md", Some(DETAILS)), file(long, Some(DETAILS))],
            44,
        );
        assert_eq!(rows[1], format!("  a.md            {DETAILS}"));
        assert_eq!(rows[4], format!("  {long}  {}", &DETAILS[..8]));
    }

    #[test]
    fn files_without_details_show_only_the_name() {
        let rows = rows(
            vec![file("a.md", None), file("longer-name.md", Some(DETAILS))],
            60,
        );
        assert_eq!(rows[1], "  a.md");
        assert_eq!(rows[4], format!("  longer-name.md  {DETAILS}"));
    }
}
