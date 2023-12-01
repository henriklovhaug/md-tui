use std::str::FromStr;

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::nodes::{MdEnum, ParseNode, ParseRoot, RenderComponent, RenderRoot};

#[derive(Parser)]
#[grammar = "md.pest"]
pub struct MdParser;

pub fn parse_markdown(file: &str) -> ParseRoot {
    let root: Pairs<'_, Rule> =
        MdParser::parse(Rule::txt, file).unwrap_or_else(|e| panic!("{}", e));

    let root_pair = root.into_iter().next().unwrap();

    ParseRoot::new(parse_text(root_pair, None))
}

fn parse_text(pair: Pair<'_, Rule>, parent: Option<MdEnum>) -> ParseNode {
    let content = pair.as_str().to_string();
    let rule = format!("{:?}", pair.as_rule());
    let kind = MdEnum::from_str(&rule).expect("Infalliable. Change when enum is complete");
    let span = pair.as_span();
    let width = (span.end() - span.start()) as u16;
    let mut component = ParseNode::new(kind, width, content, parent);
    let children = parse_children(pair.into_inner(), Some(kind));
    component.add_children(children);
    component
}

fn parse_children(pair: Pairs<'_, Rule>, parent: Option<MdEnum>) -> Vec<ParseNode> {
    let mut children = Vec::new();
    for inner_pair in pair {
        children.push(parse_text(inner_pair, parent));
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
        MdEnum::Paragraph => {
            todo!();
        }
        _ => todo!(),
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

// pub fn print_tree(component: &ParseNode, depth: usize) {
//     println!(
//         "{:depth$}{:?}: {}, height: {}, offset: {}",
//         depth,
//         component.kind(),
//         component.content(),
//         component.height(),
//         component.y_offset(),
//     );
//     for child in component.children() {
//         print_tree(child, depth + 1);
//     }
// }
