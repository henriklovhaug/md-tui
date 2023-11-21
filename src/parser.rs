use std::str::Lines;

use regex::RegexSet;

use crate::utils::Components;

pub fn parse_markdown(lines: Lines) -> Components {
    let line_regex_set = RegexSet::new(&[
        r"^#.*",                // h1
        r"^##.*",               // h2
        r"^###.*",              // h3
        r"^- \[.*\].*",         // task
        r"^-.*",                // ul
        r"^[1-9][0-9]*\..*",    // ol
        r"^>.*",                // quote
        r"^```.*",              // code block
        r"^.*",                 // paragraph
        r"^\[[a-zA-Z]\]\(.*\)", // link
        r"^\s*$",               // empty line
    ])
    .unwrap();
    for line in lines {
        match line_regex_set.matches(line) {
            _ => todo!(),
        }
    }
    todo!()
}
