use regex::RegexSet;

use crate::utils::{MdComponent, MdEnum, MdFile};

pub fn parse_markdown(lines: Vec<String>) -> MdFile {
    let line_regex_set = RegexSet::new(&[
        r"^###.*",              // h3
        r"^##.*",               // h2
        r"^#.*",                // h1
        r"^- \[.*\].*",         // task
        r"^-.*",                // ul
        r"^[1-9][0-9]*\..*",    // ol
        r"^>.*",                // quote
        r"^```.*",              // code block
        r"^\[[a-zA-Z]\]\(.*\)", // link
        r"^\s*$",               // empty line
        r"\|.*",                // table
        r"^.*",                 // paragraph
    ])
    .unwrap();

    let mut components = MdFile::new();
    for line in lines {
        let comp = match line_regex_set
            .matches(&line)
            .into_iter()
            .collect::<Vec<_>>()
            .get(0)
        {
            Some(index) => match index {
                0..=2 => MdComponent::new(MdEnum::Heading, line.to_string()),
                3 => MdComponent::new(MdEnum::Task, line.to_string()),
                4 => MdComponent::new(MdEnum::UnorderedList, line.to_string()),
                5 => MdComponent::new(MdEnum::OrderedList, line.to_string()),
                6 => MdComponent::new(MdEnum::Quote, line.to_string()),
                7 => MdComponent::new(MdEnum::CodeBlock, line.to_string()),
                8 => MdComponent::new(MdEnum::Link, line.to_string()),
                9 => MdComponent::new(MdEnum::EmptyLine, "".to_string()),
                10 => MdComponent::new(MdEnum::Table, line.to_string()),
                11 => MdComponent::new(MdEnum::Paragraph, line.to_string()),
                _ => panic!(),
            },
            None => panic!(),
        };
        components.push(comp);
    }
    components
}
