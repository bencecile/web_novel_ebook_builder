use isahc::http::Uri;
use kuchiki::{ElementData, NodeDataRef};

use crate::{
    NovelComponent, NovelError, NovelResult,
    novel::{NovelStatus},
    traverser::{TreeTraverser},
};

const FINISHED_SELECTOR: &'static str = "#noveltype";
const RUNNING_SELECTOR: &'static str = "#noveltype_notend";

pub fn fetch_status_in_info(uri: Uri) -> NovelResult<NovelStatus> {
    let node = crate::fetch_page(&uri)?;
    let info_page_data = TreeTraverser::new(node, InfoPageData::default())
        .add_hook(FINISHED_SELECTOR, None, InfoPageData::get_finished)?
        .add_hook(RUNNING_SELECTOR, None, InfoPageData::get_running)?
        .traverse();
    let status = info_page_data.status
        .ok_or(NovelError::ComponentMissing(NovelComponent::Status))?;
    Ok(status)
}

#[derive(Debug, Default)]
struct InfoPageData {
    status: Option<NovelStatus>,
}
impl InfoPageData {
    fn get_finished(&mut self, _element: &NodeDataRef<ElementData>) {
        self.status = Some(NovelStatus::Finished);
    }
    fn get_running(&mut self, _element: &NodeDataRef<ElementData>) {
        self.status = Some(NovelStatus::Running);
    }
}
