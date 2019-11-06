use kuchiki::{
    NodeRef,
};
use reqwest::{Url};

use crate::{
    NovelError, NovelComponent,
    novel::{
        Novel, Section, Chapter,
        NovelStatus,
        NovelContents,
        ContentLine, Content,
    },
};

pub fn is_kakuyomu_novel(url: &Url) -> bool {
    if let Some(host_str) = url.host_str() {
        if host_str != "kakuyomu.jp" {
            false
        } else if !url.path().starts_with("/works/") {
            false
        } else {
            true
        }
    } else {
        // We must have a host
        false
    }
}

pub fn make_kakuyomu_novel(novel_site: &str, node: NodeRef) -> Result<Novel, NovelError> {
    let title = find_title(&node)?;
    let author = find_author(&node)?;
    let status = find_status(&node)?;

    // TODO Find the sections and chapters
    let novel = Novel {
        title,
        author,
        status,
        source_url: novel_site.to_string(),
        contents: NovelContents::Sections(vec![
            Section {
                name: "SomeSection".to_string(),
                chapters: vec![
                    Chapter {
                        name: "SomeChapter".to_string(),
                        date: "Whenever".to_string(),
                        order_num: 1,
                        content: vec![
                            ContentLine::Line(vec![
                                Content::Span("Here is some content.".to_string()),
                            ]),
                        ],
                    },
                ],
            },
        ]),
    };
    Ok(novel)
}

fn find_title(node: &NodeRef) -> Result<String, NovelError> {
    let title_node = node.select_first("#workTitle > a")
        .map_err(|_| NovelError::ComponentMissing(NovelComponent::Title))?;
    Ok(title_node.text_contents())
}
fn find_author(node: &NodeRef) -> Result<String, NovelError> {
    let author_node = node.select_first("#workAuthor-activityName > a")
        .map_err(|_| NovelError::ComponentMissing(NovelComponent::Author))?;
    Ok(author_node.text_contents())
}
fn find_status(node: &NodeRef) -> Result<NovelStatus, NovelError> {
    let status_text = {
        let mut info_nodes = node.select("#workInformationList > dl > dd")
            .map_err(|_| NovelError::ComponentMissing(NovelComponent::Status))?;
        if let Some(status_node) = info_nodes.next() {
            status_node.text_contents()
        } else {
            return Err(NovelError::ComponentMissing(NovelComponent::Status));
        }
    };
    match status_text.as_str() {
        "連載中" => Ok(NovelStatus::Running),
        "完結済" => Ok(NovelStatus::Finished),
        _ => Err(NovelError::ComponentMissing(NovelComponent::Status)),
    }
}
