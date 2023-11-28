use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::utils::{MdComponent, MdComponentTree, MdEnum};

#[derive(Parser)]
#[grammar = "md.pest"]
pub struct MdParser;

pub fn parse_markdown(file: &str) -> MdComponentTree {
    let root: Pairs<'_, Rule> =
        MdParser::parse(Rule::txt, file).unwrap_or_else(|e| panic!("{}", e));

    let root_pair = root.into_iter().next().unwrap();
    let mut root_component = MdComponentTree::new(parse_component(root_pair, None));
    root_component.set_y_offset(0);

    root_component
}

fn parse_component(pair: Pair<'_, Rule>, parent: Option<MdEnum>) -> MdComponent {
    let content = pair.as_str().to_string();
    let rule = format!("{:?}", pair.as_rule());
    let kind = MdEnum::from_str(&rule);
    let span = pair.as_span();
    let width = (span.end() - span.start()) as u16;
    let mut component = MdComponent::new(kind, width, content, parent);
    let children = parse_children(pair.into_inner(), Some(kind));
    component.add_children(children);
    component
}

fn parse_children(pair: Pairs<'_, Rule>, parent: Option<MdEnum>) -> Vec<MdComponent> {
    let mut children = Vec::new();
    for inner_pair in pair {
        children.push(parse_component(inner_pair, parent));
    }
    children
}

pub fn print_tree(component: &MdComponent, depth: usize) {
    println!(
        "{:depth$}{:?}: {}, height: {}, offset: {}",
        depth,
        component.kind(),
        component.content(),
        component.height(),
        component.y_offset(),
    );
    for child in component.children() {
        print_tree(child, depth + 1);
    }
}
