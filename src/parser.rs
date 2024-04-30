use itertools::Itertools;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::nodes::{
    image::ImageComponent,
    root::{Component, ComponentRoot},
    textcomponent::{TextComponent, TextNode},
    word::{MetaData, Word, WordType},
};

#[derive(Parser)]
#[grammar = "md.pest"]
pub struct MdParser;

pub fn parse_markdown(name: Option<&str>, file: &str, width: u16) -> ComponentRoot {
    let root: Pairs<'_, Rule> = if let Ok(file) = MdParser::parse(Rule::txt, file) {
        file
    } else {
        return ComponentRoot::new(name.map(str::to_string), Vec::new());
    };

    let root_pair = root.into_iter().next().unwrap();

    let children = parse_text(root_pair).children_owned();
    let children = children.iter().dedup().cloned().collect();
    let parse_root = ParseRoot::new(name.map(str::to_string), children);

    let mut root = node_to_component(parse_root).add_missing_components();

    root.transform(width);
    root
}

fn parse_text(pair: Pair<'_, Rule>) -> ParseNode {
    let content = if pair.as_rule() != Rule::code_line {
        pair.as_str().replace('\n', " ")
    } else {
        pair.as_str().to_string()
    };
    let mut component = ParseNode::new(pair.as_rule().into(), content);
    let children = parse_node_children(pair.into_inner());
    component.add_children(children);
    component
}

fn parse_node_children(pair: Pairs<'_, Rule>) -> Vec<ParseNode> {
    let mut children = Vec::new();
    for inner_pair in pair {
        children.push(parse_text(inner_pair));
    }
    children
}

fn node_to_component(root: ParseRoot) -> ComponentRoot {
    let mut children = Vec::new();
    let name = root.file_name().to_owned();
    for component in root.children_owned() {
        let comp = parse_component(component);
        children.push(comp);
    }

    ComponentRoot::new(name, children)
}

fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn parse_component(parse_node: ParseNode) -> Component {
    match parse_node.kind() {
        MdParseEnum::Image => {
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut alt_text = String::new();
            let mut image = None;
            for node in leaf_nodes {
                if node.kind() == MdParseEnum::AltText {
                    alt_text = node.content().to_owned();
                } else if is_url(node.content()) {
                    let mut buf = Vec::new();

                    image = ureq::get(node.content()).call().ok().and_then(|b| {
                        let _ = b.into_reader().read_to_end(&mut buf);

                        image::load_from_memory(&buf).ok()
                    });
                } else {
                    image = image::io::Reader::open(node.content())
                        .ok()
                        .and_then(|r| r.decode().ok());
                }
            }

            if let Some(img) = image.as_ref() {
                let comp = ImageComponent::new(img.to_owned(), 20, alt_text);
                Component::Image(comp)
            } else {
                let word = [
                    Word::new("Image".to_string(), WordType::Normal),
                    Word::new(" ".to_owned(), WordType::Normal),
                    Word::new("not".to_owned(), WordType::Normal),
                    Word::new(" ".to_owned(), WordType::Normal),
                    Word::new("found".to_owned(), WordType::Normal),
                    Word::new("/".to_owned(), WordType::Normal),
                    Word::new("fetched".to_owned(), WordType::Normal),
                    Word::new(" ".to_owned(), WordType::Normal),
                    Word::new(format!("[{alt_text}]"), WordType::Normal),
                ];

                let comp = TextComponent::new(TextNode::Paragraph, word.into());
                Component::TextComponent(comp)
            }
        }

        MdParseEnum::Task => {
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());

                let mut content: String = node
                    .content()
                    .chars()
                    .dedup_by(|x, y| *x == ' ' && *y == ' ')
                    .collect();

                if node.kind() == MdParseEnum::WikiLink {
                    let comp = Word::new(content.clone(), WordType::LinkData);
                    words.push(comp);
                }

                if content.starts_with(' ') {
                    content.remove(0);
                    let comp = Word::new(" ".to_owned(), word_type);
                    words.push(comp);
                }
                words.push(Word::new(content, word_type));
            }
            Component::TextComponent(TextComponent::new(TextNode::Task, words))
        }

        MdParseEnum::Quote => {
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let mut content = node.content().to_owned();

                if node.kind() == MdParseEnum::WikiLink {
                    let comp = Word::new(content.clone(), WordType::LinkData);
                    words.push(comp);
                }
                if content.starts_with(' ') {
                    content.remove(0);
                    let comp = Word::new(" ".to_owned(), word_type);
                    words.push(comp);
                }
                words.push(Word::new(content, word_type));
            }
            if let Some(w) = words.first_mut() {
                w.set_content(w.content().trim_start().to_owned());
            }
            Component::TextComponent(TextComponent::new(TextNode::Quote, words))
        }

        MdParseEnum::Heading => {
            let indent = parse_node
                .content()
                .chars()
                .take_while(|c| *c == '#')
                .count();
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();

            words.push(Word::new(
                "".to_string(),
                WordType::MetaInfo(MetaData::HeadingLevel(indent as u8)),
            ));

            if indent > 1 {
                words.push(Word::new(
                    format!("{} ", "#".repeat(indent)),
                    WordType::Normal,
                ));
            }

            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let mut content = node.content().to_owned();

                if node.kind() == MdParseEnum::WikiLink {
                    let comp = Word::new(content.clone(), WordType::LinkData);
                    words.push(comp);
                }

                if content.starts_with(' ') {
                    content.remove(0);
                    let comp = Word::new(" ".to_owned(), word_type);
                    words.push(comp);
                }
                words.push(Word::new(content, word_type));
            }
            if let Some(w) = words.first_mut() {
                w.set_content(w.content().trim_start().to_owned());
            }
            Component::TextComponent(TextComponent::new(TextNode::Heading, words))
        }

        MdParseEnum::Paragraph => {
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let mut content = node.content().to_owned();

                if node.kind() == MdParseEnum::WikiLink {
                    let comp = Word::new(content.clone(), WordType::LinkData);
                    words.push(comp);
                }

                if content.starts_with(' ') {
                    content.remove(0);
                    let comp = Word::new(" ".to_owned(), word_type);
                    words.push(comp);
                }
                words.push(Word::new(content, word_type));
            }
            if let Some(w) = words.first_mut() {
                w.set_content(w.content().trim_start().to_owned());
            }
            Component::TextComponent(TextComponent::new(TextNode::Paragraph, words))
        }

        MdParseEnum::CodeBlock => {
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let content = node.content().to_owned();
                words.push(vec![Word::new(content, word_type)]);
            }
            Component::TextComponent(TextComponent::new_formatted(TextNode::CodeBlock, words))
        }

        MdParseEnum::ListContainer => {
            let mut words = Vec::new();
            for child in parse_node.children_owned() {
                let kind = child.kind();
                let leaf_nodes = get_leaf_nodes(child);
                let mut inner_words = Vec::new();
                for node in leaf_nodes {
                    let word_type = WordType::from(node.kind());

                    let mut content = match node.kind() {
                        MdParseEnum::Indent => node.content().to_owned(),
                        _ => node
                            .content()
                            .chars()
                            .dedup_by(|x, y| *x == ' ' && *y == ' ')
                            .collect(),
                    };

                    if node.kind() == MdParseEnum::WikiLink {
                        let comp = Word::new(content.clone(), WordType::LinkData);
                        inner_words.push(comp);
                    }
                    if content.starts_with(' ') && node.kind() != MdParseEnum::Indent {
                        content.remove(0);
                        let comp = Word::new(" ".to_owned(), word_type);
                        inner_words.push(comp);
                    }

                    inner_words.push(Word::new(content, word_type));
                }
                if kind == MdParseEnum::UnorderedList {
                    inner_words.push(Word::new(
                        "X".to_owned(),
                        WordType::MetaInfo(MetaData::UList),
                    ));
                    let list_symbol = Word::new("• ".to_owned(), WordType::ListMarker);
                    inner_words.insert(1, list_symbol);
                } else if kind == MdParseEnum::OrderedList {
                    inner_words.push(Word::new(
                        "X".to_owned(),
                        WordType::MetaInfo(MetaData::OList),
                    ));
                }
                words.push(inner_words);
            }
            Component::TextComponent(TextComponent::new_formatted(TextNode::List, words))
        }

        MdParseEnum::Table => {
            let mut words = Vec::new();
            for row in parse_node.children_owned() {
                if row.kind() == MdParseEnum::TableSeperator {
                    words.push(vec![Word::new(
                        row.content().to_owned(),
                        WordType::MetaInfo(MetaData::ColumnsCount),
                    )]);
                    continue;
                }
                let mut inner_words = Vec::new();
                for word in get_leaf_nodes(row) {
                    let word_type = WordType::from(word.kind());
                    let mut content = word.content().to_owned();

                    if word.kind() == MdParseEnum::WikiLink {
                        let comp = Word::new(content.clone(), WordType::LinkData);
                        inner_words.push(comp);
                    }

                    if content.starts_with(' ') {
                        content.remove(0);
                        let comp = Word::new(" ".to_owned(), word_type);
                        inner_words.push(comp);
                    }

                    inner_words.push(Word::new(content, word_type));
                }
                words.push(inner_words);
            }
            Component::TextComponent(TextComponent::new_formatted(TextNode::Table, words))
        }

        MdParseEnum::BlockSeperator => {
            Component::TextComponent(TextComponent::new(TextNode::LineBreak, Vec::new()))
        }
        MdParseEnum::HorizontalSeperator => Component::TextComponent(TextComponent::new(
            TextNode::HorizontalSeperator,
            Vec::new(),
        )),
        _ => todo!("Not implemented for {:?}", parse_node.kind()),
    }
}

fn get_leaf_nodes(node: ParseNode) -> Vec<ParseNode> {
    let mut leaf_nodes = Vec::new();
    if node.kind() == MdParseEnum::Link && node.content().starts_with(' ') {
        let comp = ParseNode::new(MdParseEnum::Word, " ".to_owned());
        leaf_nodes.push(comp);
    }

    if matches!(
        node.kind(),
        MdParseEnum::CodeStr
            | MdParseEnum::ItalicStr
            | MdParseEnum::BoldStr
            | MdParseEnum::BoldItalicStr
            | MdParseEnum::StrikethroughStr
    ) && node.content().starts_with(' ')
    {
        let comp = ParseNode::new(MdParseEnum::Word, " ".to_owned());
        leaf_nodes.push(comp);
    }

    if node.children().is_empty() {
        leaf_nodes.push(node);
    } else {
        for child in node.children_owned() {
            leaf_nodes.append(&mut get_leaf_nodes(child));
        }
    }
    leaf_nodes
}

pub fn print_from_root(root: &ComponentRoot) {
    for child in root.components() {
        print_component(child, 0);
    }
}

fn print_component(component: &TextComponent, _depth: usize) {
    println!(
        "Component: {:?}, height: {}, y_offset: {}",
        component.kind(),
        component.height(),
        component.y_offset()
    );
    component.meta_info().iter().for_each(|w| {
        println!("Meta: {}, kind: {:?}", w.content(), w.kind());
    });
    component.content().iter().for_each(|w| {
        w.iter().for_each(|w| {
            println!("Content:{}, kind: {:?}", w.content(), w.kind());
        });
    });
}

#[derive(Debug, Clone)]
pub struct ParseRoot {
    file_name: Option<String>,
    children: Vec<ParseNode>,
}

impl ParseRoot {
    pub fn new(file_name: Option<String>, children: Vec<ParseNode>) -> Self {
        Self {
            file_name,
            children,
        }
    }

    pub fn children(&self) -> &Vec<ParseNode> {
        &self.children
    }

    pub fn children_owned(self) -> Vec<ParseNode> {
        self.children
    }

    pub fn file_name(&self) -> Option<String> {
        self.file_name.to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNode {
    kind: MdParseEnum,
    content: String,
    children: Vec<ParseNode>,
}

impl ParseNode {
    pub fn new(kind: MdParseEnum, content: String) -> Self {
        Self {
            kind,
            content,
            children: Vec::new(),
        }
    }

    pub fn kind(&self) -> MdParseEnum {
        self.kind
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn add_children(&mut self, children: Vec<ParseNode>) {
        self.children.extend(children);
    }

    pub fn children(&self) -> &Vec<ParseNode> {
        &self.children
    }

    pub fn children_owned(self) -> Vec<ParseNode> {
        self.children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdParseEnum {
    AltText,
    BlockSeperator,
    Bold,
    BoldItalic,
    BoldItalicStr,
    BoldStr,
    Caution,
    Code,
    CodeBlock,
    CodeBlockStr,
    CodeStr,
    Digit,
    Heading,
    HorizontalSeperator,
    Image,
    Imortant,
    Indent,
    Italic,
    ItalicStr,
    Link,
    LinkData,
    ListContainer,
    Note,
    OrderedList,
    PLanguage,
    Paragraph,
    Quote,
    Sentence,
    Strikethrough,
    StrikethroughStr,
    Table,
    TableCell,
    TableRow,
    TableSeperator,
    Task,
    TaskClosed,
    TaskOpen,
    Tip,
    UnorderedList,
    Warning,
    WikiLink,
    Word,
}

impl From<Rule> for MdParseEnum {
    fn from(value: Rule) -> Self {
        match value {
            Rule::word | Rule::h_word | Rule::latex_word | Rule::t_word => Self::Word,
            Rule::indent => Self::Indent,
            Rule::italic_word => Self::Italic,
            Rule::italic => Self::ItalicStr,
            Rule::bold_word => Self::Bold,
            Rule::bold => Self::BoldStr,
            Rule::bold_italic_word => Self::BoldItalic,
            Rule::bold_italic => Self::BoldItalicStr,
            Rule::strikethrough_word => Self::Strikethrough,
            Rule::strikethrough => Self::StrikethroughStr,
            Rule::code_word => Self::Code,
            Rule::code => Self::CodeStr,
            Rule::programming_language => Self::PLanguage,
            Rule::link_word | Rule::link_line | Rule::link | Rule::wiki_link_word => Self::Link,
            Rule::wiki_link_alone => Self::WikiLink,
            Rule::o_list_counter | Rule::digit => Self::Digit,
            Rule::task_open => Self::TaskOpen,
            Rule::task_complete => Self::TaskClosed,
            Rule::code_line => Self::CodeBlockStr,
            Rule::sentence | Rule::t_sentence => Self::Sentence,
            Rule::table_cell => Self::TableRow,
            Rule::table_seperator => Self::TableSeperator,
            Rule::u_list => Self::UnorderedList,
            Rule::o_list => Self::OrderedList,
            Rule::h1 | Rule::h2 | Rule::h3 | Rule::h4 | Rule::h5 | Rule::h6 | Rule::heading => {
                Self::Heading
            }
            Rule::list_container => Self::ListContainer,
            Rule::paragraph => Self::Paragraph,
            Rule::code_block => Self::CodeBlock,
            Rule::table => Self::Table,
            Rule::quote => Self::Quote,
            Rule::task => Self::Task,
            Rule::block_sep => Self::BlockSeperator,
            Rule::horizontal_sep => Self::HorizontalSeperator,
            Rule::link_data | Rule::wiki_link_data => Self::LinkData,

            Rule::warning => Self::Warning,
            Rule::note => Self::Note,
            Rule::tip => Self::Tip,
            Rule::important => Self::Imortant,
            Rule::caution => Self::Caution,

            Rule::p_char
            | Rule::t_char
            | Rule::link_char
            | Rule::wiki_link_char
            | Rule::normal
            | Rule::t_normal
            | Rule::latex
            | Rule::comment
            | Rule::txt
            | Rule::task_prefix
            | Rule::quote_prefix
            | Rule::code_block_prefix
            | Rule::table_prefix
            | Rule::list_prefix
            | Rule::forbidden_sentence_prefix => Self::Paragraph,

            Rule::image => Self::Image,

            Rule::alt_word | Rule::alt_text => Self::AltText,

            Rule::heading_prefix
            | Rule::alt_char
            | Rule::b_char
            | Rule::c_char
            | Rule::c_line_char
            | Rule::comment_char
            | Rule::i_char
            | Rule::latex_char
            | Rule::quote_marking
            | Rule::s_char
            | Rule::wiki_link => todo!(),
        }
    }
}
