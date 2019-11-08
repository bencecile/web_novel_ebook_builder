use isahc::http::{Uri};
use kuchiki::{NodeData, NodeDataRef, ElementData};

use crate::{
    NovelError, NovelResult, NovelComponent,
    novel::{Content, ContentLine},
    traverser::{TreeTraverser},
};

const CONTENT_LINE_SELECTOR: &'static str = ".widget-episodeBody > p";
const BLANK_LINE_NEG: &'static str = ".widget-episodeBody > p.blank";
const BLANK_LINE_SELECTOR: &'static str = ".widget-episodeBody > p.blank > br";

pub fn fetch_novel_content(uri: Uri) -> NovelResult< Vec<ContentLine> > {
    let node = crate::fetch_page(&uri)?;
    let content_data = TreeTraverser::new(node, ContentData::default())
        .add_hook(CONTENT_LINE_SELECTOR, Some(BLANK_LINE_NEG), ContentData::get_content_line)?
        .add_hook(BLANK_LINE_SELECTOR, None, ContentData::get_blank_line)?
        .traverse();
    if content_data.lines.is_empty() {
        return Err(NovelError::ComponentMissing(NovelComponent::ChapterContent));
    }
    Ok(content_data.lines)
}

#[derive(Debug, Default)]
struct ContentData {
    lines: Vec<ContentLine>,
}
impl ContentData {
    fn get_content_line(&mut self, element: &NodeDataRef<ElementData>) {
        let mut contents = Vec::new();
        for p_child in element.as_node().children() {
            match p_child.data() {
                NodeData::Text(text) => contents.push(Content::Span(text.borrow().clone())),
                NodeData::Element(child_element) => {
                    if &child_element.name.local == "ruby" {
                        let mut main: Option<String> = None;
                        let mut above: Option<String> = None;
                        for ruby_child in p_child.children() {
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
                                contents.push(Content::Ruby {
                                    main: main.take().unwrap(),
                                    above: above.take().unwrap(),
                                });
                            }
                        }
                        if main.is_some() || above.is_some() {
                            panic!("Bad ruby. Main: {:?}. Above: {:?}", main, above);
                        }
                    }
                },
                _ => (),
            }
        }
        self.lines.push(ContentLine::Line(contents));
    }
    fn get_blank_line(&mut self, _element: &NodeDataRef<ElementData>) {
        self.lines.push(ContentLine::Blank);
    }
}
