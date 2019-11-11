use kuchiki::{ElementData, NodeData, NodeRef};

use crate::{
    novel::{Content},
};

pub fn get_ruby(node: &NodeRef, element_data: &ElementData) -> Vec<Content> {
    let mut ruby_contents = Vec::new();
    if &element_data.name.local == "ruby" {
        let mut main: Option<String> = None;
        let mut above: Option<String> = None;
        for ruby_child in node.children() {
            match ruby_child.clone().data() {
                NodeData::Text(text) => {
                    main = Some(text.borrow().clone());
                    continue;
                },
                NodeData::Element(ruby_child_element) => {
                    if &ruby_child_element.name.local == "rb" {
                        main = Some(ruby_child.text_contents());
                    } else if &ruby_child_element.name.local == "rt" {
                        above = Some(ruby_child.text_contents());
                    }
                },
                _ => (),
            }

            if main.is_some() && above.is_some() {
                ruby_contents.push(Content::Ruby {
                    main: main.take().unwrap(),
                    above: above.take().unwrap(),
                });
            }
        }
        if main.is_some() || above.is_some() {
            panic!("Bad ruby. Main: {:?}. Above: {:?}", main, above);
        }
    }
    ruby_contents
}

pub fn convert_num_string_to_ja(num_string: &str) -> String {
    num_string.chars().map(|c| match c {
        '0' => '〇',
        '1' => '一',
        '2' => '二',
        '3' => '三',
        '4' => '四',
        '5' => '五',
        '6' => '六',
        '7' => '七',
        '8' => '八',
        '9' => '九',
        _ => c,
    }).collect()
}
