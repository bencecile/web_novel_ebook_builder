use isahc::http::{Uri};
use kuchiki::{NodeData, NodeDataRef, ElementData};

use crate::{
    NovelError, NovelResult, NovelComponent,
    novel::{
        Content, ContentLine,
        novel_utils,
    },
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
    fn get_content_line(&mut self, element: &NodeDataRef<ElementData>) {
        let mut contents = Vec::new();
        for p_child in element.as_node().children() {
            match p_child.data() {
                NodeData::Text(text) => contents.push(Content::Span(text.borrow().clone())),
                NodeData::Element(child_element) => {
                    let mut ruby_contents = novel_utils::get_ruby(&p_child, &child_element);
                    contents.append(&mut ruby_contents);
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
