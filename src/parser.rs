use std::str::FromStr;

use itertools::Itertools;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::nodes::{MetaData, RenderComponent, RenderNode, RenderRoot, Word, WordType};

#[derive(Parser)]
#[grammar = "md.pest"]
pub struct MdParser;

pub fn parse_markdown(name: &str, file: &str) -> RenderRoot {
    let root: Pairs<'_, Rule> =
        MdParser::parse(Rule::txt, file).unwrap_or_else(|e| panic!("{}", e));

    let root_pair = root.into_iter().next().unwrap();

    let children = parse_text(root_pair).children_owned();
    let children = children.iter().dedup().cloned().collect();
    let parse_root = ParseRoot::new(name.to_owned(), children);

    node_to_component(parse_root)
}

fn parse_text(pair: Pair<'_, Rule>) -> ParseNode {
    let content = pair.as_str().replace('\n', " ");
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

fn node_to_component(root: ParseRoot) -> RenderRoot {
    let mut children = Vec::new();
    let name = root.file_name().to_owned();
    for component in root.children_owned() {
        let comp = parse_component(component);
        children.push(comp);
    }

    RenderRoot::new(name, children)
}

fn parse_component(parse_node: ParseNode) -> RenderComponent {
    match parse_node.kind() {
        MdParseEnum::Task => {
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let content = node
                    .content()
                    .chars()
                    .dedup_by(|x, y| *x == ' ' && *y == ' ')
                    .collect();
                words.push(Word::new(content, word_type));
            }
            RenderComponent::new(RenderNode::Task, words)
        }

        MdParseEnum::Paragraph | MdParseEnum::Heading | MdParseEnum::Quote => {
            let kind = parse_node.kind();
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let mut content = node.content().to_owned();
                if content.ends_with(' ') {
                    content.pop();
                    words.push(Word::new(content, word_type));
                    let comp = Word::new(" ".to_owned(), word_type);
                    words.push(comp);
                } else {
                    words.push(Word::new(content, word_type));
                }
            }
            match kind {
                MdParseEnum::Paragraph => RenderComponent::new(RenderNode::Paragraph, words),
                MdParseEnum::Heading => RenderComponent::new(RenderNode::Heading, words),
                MdParseEnum::Quote => RenderComponent::new(RenderNode::Quote, words),
                _ => unreachable!(),
            }
        }

        MdParseEnum::CodeBlock => {
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let content = node.content().to_owned();
                words.push(vec![Word::new(content, word_type)]);
            }
            RenderComponent::new_formatted(RenderNode::CodeBlock, words)
        }

        MdParseEnum::ListContainer => {
            let mut words = Vec::new();
            for child in parse_node.children_owned() {
                let kind = child.kind();
                let leaf_nodes = get_leaf_nodes(child);
                let mut inner_words = Vec::new();
                for node in leaf_nodes {
                    let word_type = WordType::from(node.kind());

                    let content = match node.kind() {
                        MdParseEnum::Indent => node.content().to_owned(),
                        _ => node
                            .content()
                            .chars()
                            .dedup_by(|x, y| *x == ' ' && *y == ' ')
                            .collect(),
                    };
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
            RenderComponent::new_formatted(RenderNode::List, words)
        }

        MdParseEnum::Table => {
            let mut words = Vec::new();
            for row in parse_node.children_owned() {
                if row.kind() == MdParseEnum::TableSeperator {
                    continue;
                }
                let mut inner_words = Vec::new();
                for word in get_leaf_nodes(row) {
                    let word_type = WordType::from(word.kind());
                    let content = word.content().to_owned();
                    inner_words.push(Word::new(content, word_type));
                }
                words.push(inner_words);
            }
            RenderComponent::new_formatted(RenderNode::Table, words)
        }

        MdParseEnum::BlockSeperator => RenderComponent::new(RenderNode::LineBreak, Vec::new()),
        _ => todo!("Not implemented for {:?}", parse_node.kind()),
    }
}

fn get_leaf_nodes(node: ParseNode) -> Vec<ParseNode> {
    let mut leaf_nodes = Vec::new();
    if (node.kind() == MdParseEnum::Code
        || node.kind() == MdParseEnum::Italic
        || node.kind() == MdParseEnum::Strikethrough
        || node.kind() == MdParseEnum::Bold
        || node.kind() == MdParseEnum::Link)
        && node.content().starts_with(' ')
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

pub fn print_from_root(root: &RenderRoot) {
    for child in root.components() {
        print_component(child, 0);
    }
}

fn print_component(component: &RenderComponent, _depth: usize) {
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
            println!("Content: {}, kind: {:?}", w.content(), w.kind());
        });
    });
}

#[derive(Debug, Clone)]
pub struct ParseRoot {
    file_name: String,
    children: Vec<ParseNode>,
}

impl ParseRoot {
    pub fn new(file_name: String, children: Vec<ParseNode>) -> Self {
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

    pub fn file_name(&self) -> &str {
        &self.file_name
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

    pub fn children_mut(&mut self) -> &mut Vec<ParseNode> {
        &mut self.children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdParseEnum {
    Heading,
    Word,
    Task,
    TaskOpen,
    TaskClosed,
    UnorderedList,
    ListContainer,
    OrderedList,
    CodeBlock,
    PLanguage,
    CodeStr,
    Code,
    Paragraph,
    Link,
    LinkData,
    Quote,
    Table,
    TableSeperator,
    TableRow,
    Digit,
    BlockSeperator,
    Sentence,
    Bold,
    Italic,
    Strikethrough,
    HorizontalSeperator,
    Indent,
}

impl From<Rule> for MdParseEnum {
    fn from(value: Rule) -> Self {
        match value {
            Rule::word | Rule::table_word => Self::Word,
            Rule::indent => Self::Indent,
            Rule::italic | Rule::italic_word => Self::Italic,
            Rule::bold | Rule::bold_word => Self::Bold,
            Rule::strikethrough | Rule::strikethrough_word => Self::Strikethrough,
            Rule::code_word | Rule::code => Self::Code,
            Rule::programming_language => Self::PLanguage,
            Rule::link_word | Rule::markdown_link | Rule::external_link | Rule::link => Self::Link,
            Rule::o_list_counter | Rule::digit => Self::Digit,
            Rule::task_open => Self::TaskOpen,
            Rule::task_complete => Self::TaskClosed,
            Rule::code_line | Rule::sentence => Self::Sentence,
            Rule::table_row => Self::TableRow,
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
            Rule::link_data => Self::LinkData,

            Rule::norwegian_char
            | Rule::p_char
            | Rule::table_char
            | Rule::link_char
            | Rule::normal
            | Rule::comment
            | Rule::txt
            | Rule::task_prefix
            | Rule::quote_prefix
            | Rule::code_block_prefix
            | Rule::table_prefix
            | Rule::list_prefix
            | Rule::forbidden_sentence_prefix => Self::Paragraph,

            Rule::heading_prefix
            | Rule::c_char
            | Rule::i_char
            | Rule::b_char
            | Rule::s_char
            | Rule::comment_char
            | Rule::c_line_char => todo!(),
        }
    }
}

impl FromStr for MdParseEnum {
    type Err = ();

    /// This cannot return Err
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "h1" | "h2" | "h3" | "h4" | "heading" => Ok(Self::Heading),
            "task" => Ok(Self::Task),
            "task_open" => Ok(Self::TaskOpen),
            "task_complete" => Ok(Self::TaskClosed),
            "u_list" => Ok(Self::UnorderedList),
            "o_list" => Ok(Self::OrderedList),
            "code_block" => Ok(Self::CodeBlock),
            "programming_language" => Ok(Self::PLanguage),
            "code" | "code_word" => Ok(Self::Code),
            "paragraph" => Ok(Self::Paragraph),
            "link" => Ok(Self::Link),
            "quote" => Ok(Self::Quote),
            "table" => Ok(Self::Table),
            "table_seperator" => Ok(Self::TableSeperator),
            "table_row" => Ok(Self::TableRow),
            "block_sep" => Ok(Self::BlockSeperator),
            "code_line" => Ok(Self::Sentence),
            "list_container" => Ok(Self::ListContainer),
            "table_word" | "o_list_counter" | "word" | "digit" => Ok(Self::Word),
            "bold" | "bold_word" => Ok(Self::Bold),
            "italic" | "italic_word" => Ok(Self::Italic),
            "strikethrough" | "strikethrough_word" => Ok(Self::Strikethrough),
            _e => Ok(Self::Paragraph),
        }
    }
}
