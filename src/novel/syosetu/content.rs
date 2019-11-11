use isahc::http::Uri;
use kuchiki::{ElementData, NodeData, NodeDataRef};

use crate::{
    NovelComponent, NovelError, NovelResult,
    novel::{
        Content, ContentLine,
        novel_utils,
    },
    traverser::{TreeTraverser},
};

const LINE_SELECTOR: &'static str = "#novel_honbun > p";
const BLANK_SELECTOR: &'static str = "#novel_honbun > p > br";

pub fn fetch_page_content(uri: Uri) -> NovelResult< Vec<ContentLine> > {
    let page_node = crate::fetch_page(&uri)?;
    let content_data = TreeTraverser::new(page_node, ContentData::default())
        .add_hook(LINE_SELECTOR, None, ContentData::get_line)?
        .add_hook(BLANK_SELECTOR, None, ContentData::get_blank)?
        .traverse();
    if content_data.lines.is_empty() {
        println!("Couldn't get contents of {:?}", &uri);
        return Err(NovelError::ComponentMissing(NovelComponent::ChapterContent));
    }
    Ok(content_data.lines)
}

#[derive(Debug, Default)]
struct ContentData {
    lines: Vec<ContentLine>,
}
impl ContentData {
    fn get_line(&mut self, element: &NodeDataRef<ElementData>) {
        let mut contents: Vec<Content> = Vec::new();
        for child in element.as_node().children() {
            match child.data() {
                NodeData::Text(text) => contents.push(Content::Span(text.borrow().to_string())),
                NodeData::Element(child_element) => {
                    let mut ruby_contents = novel_utils::get_ruby(&child, &child_element);
                    contents.append(&mut ruby_contents);
                },
                _ => (),
            }
        }
        if !contents.is_empty() {
            self.lines.push(ContentLine::Line(contents));
        }
    }
    fn get_blank(&mut self, _element: &NodeDataRef<ElementData>) {
        self.lines.push(ContentLine::Blank);
    }
}
