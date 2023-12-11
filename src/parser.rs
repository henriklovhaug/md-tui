use itertools::Itertools;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::nodes::{
    MdParseEnum, ParseNode, ParseRoot, RenderComponent, RenderNode, RenderRoot, Word, WordType,
};

#[derive(Parser)]
#[grammar = "md.pest"]
pub struct MdParser;

pub fn parse_markdown(file: &str) -> RenderRoot {
    let root: Pairs<'_, Rule> =
        MdParser::parse(Rule::txt, file).unwrap_or_else(|e| panic!("{}", e));

    let root_pair = root.into_iter().next().unwrap();

    let children = parse_text(root_pair).children_owned();
    let children = children.iter().dedup().cloned().collect();
    let parse_root = ParseRoot::new(children);

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
    for component in root.children_owned() {
        let comp = parse_component(component);
        children.push(comp);
    }

    RenderRoot::new(children)
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

        MdParseEnum::Paragraph | MdParseEnum::Heading => {
            let kind = parse_node.kind();
            let leaf_nodes = get_leaf_nodes(parse_node);
            let mut words = Vec::new();
            for node in leaf_nodes {
                let word_type = WordType::from(node.kind());
                let content = node.content().to_owned();
                words.push(Word::new(content, word_type));
            }
            match kind {
                MdParseEnum::Paragraph => RenderComponent::new(RenderNode::Paragraph, words),
                MdParseEnum::Heading => RenderComponent::new(RenderNode::Heading, words),
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
                let leaf_nodes = get_leaf_nodes(child);
                let mut inner_words = Vec::new();
                for node in leaf_nodes {
                    let word_type = WordType::from(node.kind());
                    let content = node.content().to_owned();
                    inner_words.push(Word::new(content, word_type));
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
    component.content().iter().for_each(|w| {
        w.iter().for_each(|w| {
            println!("Content: {}, kind: {:?}", w.content(), w.kind());
        });
    });
}
