use std::sync::atomic::{AtomicU32, Ordering};

use image::ImageReader;
use itertools::Itertools;
use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;
use ratatui::style::Color;

use crate::nodes::{
    image::ImageComponent,
    root::{Component, ComponentRoot},
    textcomponent::{TextComponent, TextNode},
    word::{MetaData, Word, WordType},
};

/// Process-wide monotonic counter for assigning unique IDs to `<details>`
/// blocks. Each parsed details summary gets a fresh ID so it can be addressed
/// by the runtime fold-toggle and selector independently of its position in
/// the document.
static DETAILS_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

fn next_details_id() -> u32 {
    DETAILS_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Prepend `id` to the owning-details chain of every text component in
/// `components`. Used after parsing a `<details>` body so that nested
/// children (which may already carry inner IDs) correctly record the
/// outer-to-inner containment order.
fn tag_owning_details(components: &mut [Component], id: u32) {
    for c in components.iter_mut() {
        if let Component::TextComponent(tc) = c {
            tc.prepend_owning_details_id(id);
        }
    }
}

#[derive(Parser)]
#[grammar = "md.pest"]
pub struct MdParser;

pub fn parse_markdown(name: Option<&str>, content: &str, width: u16) -> ComponentRoot {
    let root: Pairs<'_, Rule> = if let Ok(file) = MdParser::parse(Rule::txt, content) {
        file
    } else {
        return ComponentRoot::new(name.map(str::to_string), Vec::new());
    };

    let root_pair = root.into_iter().next().unwrap();

    let children = parse_text(root_pair)
        .children_owned()
        .into_iter()
        .dedup()
        .collect();

    let parse_root = ParseRoot::new(name.map(str::to_string), children);

    let mut root = node_to_component(parse_root).add_missing_components();

    root.transform(width);
    root.recompute_visibility();
    root
}

fn parse_text(pair: Pair<'_, Rule>) -> ParseNode {
    let content = if pair.as_rule() == Rule::code_line {
        pair.as_str().replace('\t', "    ").replace('\r', "")
    } else {
        pair.as_str().replace('\n', " ")
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
    let name = root.file_name().clone();
    for component in root.children_owned() {
        children.extend(parse_components(component));
    }

    ComponentRoot::new(name, children)
}

fn parse_components(parse_node: ParseNode) -> Vec<Component> {
    if parse_node.kind() == MdParseEnum::Details {
        return parse_details(parse_node);
    }
    vec![parse_component(parse_node)]
}

fn parse_details(parse_node: ParseNode) -> Vec<Component> {
    let mut header_text = String::from("Details");
    let mut body_components: Vec<Component> = Vec::new();
    let mut open_attr_present = false;

    for child in parse_node.children_owned() {
        match child.kind() {
            MdParseEnum::DetailsOpenAttr => {
                open_attr_present = true;
            }
            MdParseEnum::DetailsSummary => {
                let text: String = get_leaf_nodes(child)
                    .into_iter()
                    .map(|n| n.content().to_string())
                    .collect::<Vec<_>>()
                    .join("");
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    header_text = trimmed;
                }
            }
            MdParseEnum::DetailsBody => {
                for body_child in child.children_owned() {
                    body_components.extend(parse_components(body_child));
                }
            }
            _ => {
                body_components.extend(parse_components(child));
            }
        }
    }

    let id = next_details_id();
    tag_owning_details(&mut body_components, id);

    let body_len = body_components.len();
    let folded = !open_attr_present;

    let mut out = Vec::with_capacity(1 + body_len);
    out.push(Component::TextComponent(TextComponent::new(
        TextNode::DetailsSummary {
            id,
            folded,
            body_len,
        },
        vec![Word::new(header_text, WordType::Normal)],
    )));
    out.extend(body_components);
    out
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
                    node.content().clone_into(&mut alt_text);
                } else if is_url(node.content()) {
                    #[cfg(feature = "network")]
                    {
                        let mut buf = Vec::new();
                        image = ureq::get(node.content()).call().ok().and_then(|b| {
                            let noe = b.into_body().read_to_vec();
                            noe.ok().and_then(|b| {
                                buf = b;
                                image::load_from_memory(&buf).ok()
                            })
                        });
                    }
                    #[cfg(not(feature = "network"))]
                    {
                        image = None;
                    }
                } else {
                    image = ImageReader::open(node.content())
                        .ok()
                        .and_then(|r| r.decode().ok());
                }
            }

            if let Some(img) = image.as_ref() {
                let height = img.height();

                let comp = ImageComponent::new(img.to_owned(), height, alt_text.clone());

                if let Some(comp) = comp {
                    Component::Image(comp)
                } else {
                    let word = [Word::new(format!("[{alt_text}]"), WordType::Normal)];

                    let comp = TextComponent::new(TextNode::Paragraph, word.into());
                    Component::TextComponent(comp)
                }
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

                if matches!(node.kind(), MdParseEnum::WikiLink | MdParseEnum::InlineLink) {
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

                if matches!(node.kind(), MdParseEnum::WikiLink | MdParseEnum::InlineLink) {
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
                String::new(),
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

                if matches!(node.kind(), MdParseEnum::WikiLink | MdParseEnum::InlineLink) {
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

                if matches!(node.kind(), MdParseEnum::WikiLink | MdParseEnum::InlineLink) {
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

            let mut space_indented = false;

            for node in leaf_nodes {
                if node.kind() == MdParseEnum::CodeBlockStrSpaceIndented {
                    space_indented = true;
                }
                let word_type = WordType::from(node.kind());
                let content = node.content().to_owned();
                words.push(vec![Word::new(content, word_type)]);
            }

            if space_indented {
                words.push(vec![Word::new(
                    " ".to_owned(),
                    WordType::CodeBlock(Color::Reset),
                )]);
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

                    if matches!(node.kind(), MdParseEnum::WikiLink | MdParseEnum::InlineLink) {
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
            let mut meta_info = Vec::new();
            for cell in parse_node.children_owned() {
                if cell.kind() == MdParseEnum::TableSeparator {
                    meta_info.push(Word::new(
                        cell.content().to_owned(),
                        WordType::MetaInfo(MetaData::ColumnsCount),
                    ));
                    continue;
                }
                let mut inner_words = Vec::new();

                if cell.children().is_empty() {
                    words.push(inner_words);
                    continue;
                }

                for word in get_leaf_nodes(cell) {
                    let word_type = WordType::from(word.kind());
                    let mut content = word.content().to_owned();

                    if matches!(word.kind(), MdParseEnum::WikiLink | MdParseEnum::InlineLink) {
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
            Component::TextComponent(TextComponent::new_formatted_with_meta(
                TextNode::Table(vec![], vec![]),
                words,
                meta_info,
            ))
        }

        MdParseEnum::BlockSeparator => {
            Component::TextComponent(TextComponent::new(TextNode::LineBreak, Vec::new()))
        }
        MdParseEnum::HorizontalSeparator => Component::TextComponent(TextComponent::new(
            TextNode::HorizontalSeparator,
            Vec::new(),
        )),
        MdParseEnum::Footnote => {
            let mut words = Vec::new();
            let foot_ref = parse_node.children().first().unwrap().to_owned();
            words.push(Word::new(foot_ref.content, WordType::FootnoteData));
            let _rest = parse_node
                .children_owned()
                .into_iter()
                .skip(1)
                .map(|e| e.content)
                .collect::<String>();
            words.push(Word::new(_rest, WordType::Footnote));
            Component::TextComponent(TextComponent::new(TextNode::Footnote, words))
        }
        _ => todo!("Not implemented for {:?}", parse_node.kind()),
    }
}

fn get_leaf_nodes(node: ParseNode) -> Vec<ParseNode> {
    let mut leaf_nodes = Vec::new();

    // Insert separator information between links
    if node.kind() == MdParseEnum::Link {
        let comp = if node.content().starts_with(' ') {
            ParseNode::new(MdParseEnum::Word, " ".to_owned())
        } else {
            ParseNode::new(MdParseEnum::Word, String::new())
        };
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
    #[must_use]
    pub fn new(file_name: Option<String>, children: Vec<ParseNode>) -> Self {
        Self {
            file_name,
            children,
        }
    }

    #[must_use]
    pub fn children(&self) -> &Vec<ParseNode> {
        &self.children
    }

    #[must_use]
    pub fn children_owned(self) -> Vec<ParseNode> {
        self.children
    }

    #[must_use]
    pub fn file_name(&self) -> Option<String> {
        self.file_name.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNode {
    kind: MdParseEnum,
    content: String,
    children: Vec<ParseNode>,
}

impl ParseNode {
    #[must_use]
    pub fn new(kind: MdParseEnum, content: String) -> Self {
        Self {
            kind,
            content,
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> MdParseEnum {
        self.kind
    }

    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn add_children(&mut self, children: Vec<ParseNode>) {
        self.children.extend(children);
    }

    #[must_use]
    pub fn children(&self) -> &Vec<ParseNode> {
        &self.children
    }

    #[must_use]
    pub fn children_owned(self) -> Vec<ParseNode> {
        self.children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdParseEnum {
    AltText,
    BlockSeparator,
    Bold,
    BoldItalic,
    BoldItalicStr,
    BoldStr,
    Caution,
    Code,
    CodeBlock,
    CodeBlockStr,
    CodeBlockStrSpaceIndented,
    CodeStr,
    Details,
    DetailsBody,
    DetailsOpenAttr,
    DetailsSummary,
    Digit,
    FootnoteRef,
    Footnote,
    Heading,
    HorizontalSeparator,
    Image,
    Imortant,
    Indent,
    InlineLink,
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
    TableSeparator,
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
            Rule::italic_word_var_1 | Rule::italic_word_var_2 => Self::Italic,
            Rule::italic_var_1 | Rule::italic_var_2 => Self::ItalicStr,
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
            Rule::inline_link | Rule::inline_link_wrapper => Self::InlineLink,
            Rule::o_list_counter | Rule::digit => Self::Digit,
            Rule::task_open => Self::TaskOpen,
            Rule::task_complete => Self::TaskClosed,
            Rule::code_line => Self::CodeBlockStr,
            Rule::indented_code_line | Rule::indented_code_newline => {
                Self::CodeBlockStrSpaceIndented
            }
            Rule::sentence | Rule::t_sentence | Rule::footnote_sentence => Self::Sentence,
            Rule::table_cell => Self::TableCell,
            Rule::table_separator => Self::TableSeparator,
            Rule::u_list => Self::UnorderedList,
            Rule::o_list => Self::OrderedList,
            Rule::h1 | Rule::h2 | Rule::h3 | Rule::h4 | Rule::h5 | Rule::h6 | Rule::heading => {
                Self::Heading
            }
            Rule::list_container => Self::ListContainer,
            Rule::paragraph => Self::Paragraph,
            Rule::code_block | Rule::indented_code_block => Self::CodeBlock,
            Rule::table => Self::Table,
            Rule::quote => Self::Quote,
            Rule::task => Self::Task,
            Rule::block_sep => Self::BlockSeparator,
            Rule::horizontal_sep => Self::HorizontalSeparator,
            Rule::link_data | Rule::wiki_link_data => Self::LinkData,
            Rule::details => Self::Details,
            Rule::details_body => Self::DetailsBody,
            Rule::details_open_attr => Self::DetailsOpenAttr,
            Rule::summary | Rule::summary_text => Self::DetailsSummary,
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
            Rule::footnote_ref => Self::FootnoteRef,
            Rule::footnote => Self::Footnote,
            Rule::heading_prefix
            | Rule::alt_char
            | Rule::b_char
            | Rule::c_char
            | Rule::c_line_char
            | Rule::comment_char
            | Rule::i_char_var_1
            | Rule::i_char_var_2
            | Rule::latex_char
            | Rule::quote_marking
            | Rule::inline_link_char
            | Rule::s_char
            | Rule::WHITESPACE_S
            | Rule::wiki_link
            | Rule::footnote_ref_container
            | Rule::details_open_tag
            | Rule::details_close_tag
            | Rule::summary_open_tag
            | Rule::summary_close_tag => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::textcomponent::TextNode;

    fn component_kinds(md: &str) -> Vec<TextNode> {
        parse_markdown(None, md, 80)
            .components()
            .iter()
            .map(|c| c.kind())
            .collect()
    }

    fn has_details_summary(kinds: &[TextNode]) -> bool {
        kinds
            .iter()
            .any(|k| matches!(k, TextNode::DetailsSummary { .. }))
    }

    #[test]
    fn parses_details_with_summary() {
        let md = "<details>\n<summary>Title</summary>\n\nBody paragraph.\n\n</details>\n";
        let kinds = component_kinds(md);
        assert!(
            has_details_summary(&kinds),
            "expected DetailsSummary header, got {kinds:?}"
        );
        assert!(
            kinds.iter().any(|k| matches!(k, TextNode::Paragraph)),
            "expected body paragraph, got {kinds:?}"
        );
    }

    #[test]
    fn parses_details_open_attribute_starts_unfolded() {
        // `<details open>` honors HTML semantics: initial state expanded.
        let md = "<details open>\n<summary>S</summary>\n\nbody\n\n</details>\n";
        let kinds = component_kinds(md);
        let folded = kinds.iter().find_map(|k| match k {
            TextNode::DetailsSummary { folded, .. } => Some(*folded),
            _ => None,
        });
        assert_eq!(
            folded,
            Some(false),
            "<details open> should start unfolded, got {kinds:?}"
        );
    }

    #[test]
    fn parses_details_without_open_starts_folded() {
        // Plain `<details>` (no `open` attribute) starts collapsed.
        let md = "<details>\n<summary>S</summary>\n\nbody\n\n</details>\n";
        let kinds = component_kinds(md);
        let folded = kinds.iter().find_map(|k| match k {
            TextNode::DetailsSummary { folded, .. } => Some(*folded),
            _ => None,
        });
        assert_eq!(
            folded,
            Some(true),
            "<details> without `open` should start folded, got {kinds:?}"
        );
    }

    #[test]
    fn parses_details_without_summary() {
        let md = "<details>\n\nplain body\n\n</details>\n";
        let kinds = component_kinds(md);
        assert!(has_details_summary(&kinds));
    }

    #[test]
    fn parses_uppercase_details() {
        let md = "<DETAILS>\n<SUMMARY>Caps</SUMMARY>\n\nbody\n\n</DETAILS>\n";
        let kinds = component_kinds(md);
        assert!(
            has_details_summary(&kinds),
            "case-insensitive matching failed, got {kinds:?}"
        );
    }

    #[test]
    fn malformed_details_does_not_panic() {
        let md = "<details>\n<summary>S</summary>\n\nbody never closes\n";
        let _ = parse_markdown(None, md, 80);
    }

    #[test]
    fn nested_details_produces_two_summary_headers() {
        let md = "<details>\n<summary>Outer</summary>\n\n<details>\n<summary>Inner</summary>\n\ninner body\n\n</details>\n\n</details>\n";
        let kinds = component_kinds(md);
        let summary_count = kinds
            .iter()
            .filter(|k| matches!(k, TextNode::DetailsSummary { .. }))
            .count();
        assert_eq!(summary_count, 2, "expected 2 DetailsSummary, got {kinds:?}");
    }

    #[test]
    fn html_close_tag_not_autolink() {
        let md = "</details>";
        let kinds = component_kinds(md);
        assert!(
            kinds
                .iter()
                .all(|k| !matches!(k, TextNode::DetailsSummary { .. })),
            "stray close tag shouldn't produce DetailsSummary"
        );
    }

    #[test]
    fn issue_169_example_parses() {
        // The exact example from issue #169 — two <details> blocks with tables.
        let md = "# Dependencies\n\n\
            <details>\n<summary>Explicit dependencies</summary>\n\n\
            |Dependency|Before|After|\n|-|-|-|\n|bpy|0.10.1|2.10.1|\n\n\
            </details>\n\n\
            <details open>\n<summary>Implicit dependencies</summary>\n\n\
            |Dependency|Before|After|\n|-|-|-|\n|python|0.10.0|0.10.1|\n\n\
            </details>\n";
        let kinds = component_kinds(md);
        let summary_count = kinds
            .iter()
            .filter(|k| matches!(k, TextNode::DetailsSummary { .. }))
            .count();
        assert_eq!(
            summary_count, 2,
            "expected 2 summary headers, got {kinds:?}"
        );
        let table_count = kinds
            .iter()
            .filter(|k| matches!(k, TextNode::Table(_, _)))
            .count();
        assert_eq!(
            table_count, 2,
            "expected 2 tables inside details, got {kinds:?}"
        );
    }

    #[test]
    fn plain_paragraph_unaffected() {
        let md = "Just a paragraph.\n";
        let kinds = component_kinds(md);
        assert!(!has_details_summary(&kinds));
    }

    #[test]
    fn nested_details_tags_inner_components_with_both_ids() {
        let md = "<details>\n<summary>Outer</summary>\n\n<details>\n<summary>Inner</summary>\n\ninner body\n\n</details>\n\n</details>\n";
        let root = parse_markdown(None, md, 80);
        let comps = root.components();
        // Find the inner summary's owning chain — it should have exactly
        // one id (the outer's). The inner body should have two (outer,
        // inner — outermost-first ordering).
        let summaries: Vec<&[u32]> = comps
            .iter()
            .filter(|c| matches!(c.kind(), TextNode::DetailsSummary { .. }))
            .map(|c| c.owning_details_ids())
            .collect();
        assert_eq!(summaries.len(), 2, "expected 2 summaries");
        // First (outer) summary has no owning details. Second (inner)
        // summary has one: the outer.
        assert_eq!(summaries[0].len(), 0, "outer summary has no owners");
        assert_eq!(
            summaries[1].len(),
            1,
            "inner summary belongs to one outer details body"
        );

        // The inner body paragraph belongs to both outer + inner.
        let inner_para = comps
            .iter()
            .find(|c| matches!(c.kind(), TextNode::Paragraph) && c.owning_details_ids().len() == 2)
            .expect("inner body paragraph with two owning details ids");
        assert_eq!(
            inner_para.owning_details_ids().len(),
            2,
            "inner body paragraph belongs to outer and inner"
        );
    }

    #[test]
    fn default_collapsed_hides_body_components() {
        // A plain (collapsed-by-default) <details> hides its body
        // components, so they contribute zero height to the layout.
        let md = "<details>\n<summary>S</summary>\n\nhidden body\n\n</details>\n";
        let root = parse_markdown(None, md, 80);
        let comps = root.components();
        let body_para = comps
            .iter()
            .find(|c| matches!(c.kind(), TextNode::Paragraph))
            .expect("expected body paragraph component");
        assert!(body_para.is_hidden(), "collapsed body should be hidden");
        assert_eq!(
            body_para.height(),
            0,
            "hidden component height must be 0 so set_scroll positions correctly"
        );
    }

    #[test]
    fn open_attribute_keeps_body_visible() {
        let md = "<details open>\n<summary>S</summary>\n\nvisible body\n\n</details>\n";
        let root = parse_markdown(None, md, 80);
        let comps = root.components();
        let body_para = comps
            .iter()
            .find(|c| matches!(c.kind(), TextNode::Paragraph))
            .expect("expected body paragraph component");
        assert!(!body_para.is_hidden(), "open body should be visible");
    }

    #[test]
    fn toggle_fold_hides_and_reveals_body() {
        let md = "<details open>\n<summary>S</summary>\n\nbody text\n\n</details>\n";
        let mut root = parse_markdown(None, md, 80);
        let initial_height = root.height();
        // Select the only details summary, then toggle it folded.
        root.select_details(0).expect("select_details");
        root.toggle_selected_details().expect("toggle");
        let folded_height = root.height();
        assert!(
            folded_height < initial_height,
            "folding should reduce total height ({folded_height} < {initial_height})"
        );
        // Toggle again to re-expand.
        root.toggle_selected_details().expect("untoggle");
        let unfolded_height = root.height();
        assert_eq!(
            unfolded_height, initial_height,
            "unfolding restores original height"
        );
    }

    #[test]
    fn outer_fold_hides_inner_summary() {
        let md = "<details open>\n<summary>Outer</summary>\n\n<details open>\n<summary>Inner</summary>\n\ninner body\n\n</details>\n\n</details>\n";
        let mut root = parse_markdown(None, md, 80);
        // Fold the outer details — the inner summary header AND its body
        // should both become hidden.
        root.select_details(0).expect("select outer");
        root.toggle_selected_details().expect("fold outer");

        let mut inner_summary_hidden = false;
        let mut inner_body_hidden = false;
        for c in root.components() {
            if matches!(c.kind(), TextNode::DetailsSummary { .. })
                && c.owning_details_ids().len() == 1
                && c.is_hidden()
            {
                inner_summary_hidden = true;
            }
            if matches!(c.kind(), TextNode::Paragraph)
                && c.owning_details_ids().len() == 2
                && c.is_hidden()
            {
                inner_body_hidden = true;
            }
        }
        assert!(
            inner_summary_hidden,
            "inner summary should be hidden when outer is folded"
        );
        assert!(
            inner_body_hidden,
            "inner body should be hidden when outer is folded"
        );

        // num_details reports only currently-visible summaries — the
        // inner one disappears from the selector cycle while outer is
        // folded.
        assert_eq!(
            root.num_details(),
            1,
            "only the outer summary is visible when outer is folded"
        );
    }

    #[test]
    fn linebreak_inherits_shared_owning_ids() {
        // The block-separator-inserted LineBreaks should inherit the
        // owning-details chain that is shared between their neighbors,
        // so a LineBreak between two body components is hidden together
        // with them when the surrounding details folds.
        let md = "<details>\n<summary>S</summary>\n\nfirst body\n\nsecond body\n\n</details>\n";
        let root = parse_markdown(None, md, 80);
        let comps = root.components();
        let interior_linebreak = comps.iter().find(|c| {
            matches!(c.kind(), TextNode::LineBreak) && !c.owning_details_ids().is_empty()
        });
        assert!(
            interior_linebreak.is_some(),
            "expected a LineBreak inside the details body to inherit its owners"
        );
        let lb = interior_linebreak.unwrap();
        assert!(
            lb.is_hidden(),
            "LineBreak inside a folded details body should be hidden"
        );
    }
}
