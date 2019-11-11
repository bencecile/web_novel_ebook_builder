mod content;
mod info_page;

use isahc::http::{Uri};
use kuchiki::{ElementData, NodeDataRef};

use crate::{
    NovelComponent, NovelError, NovelResult,
    novel::{Novel, Section, Chapter, NovelContents},
    traverser::{TreeTraverser},
};

const HOST_NAME: &'static str = "ncode.syosetu.com";
fn make_uri(path: &str) -> NovelResult<Uri> {
    if path.starts_with("http") {
        Ok(path.parse()?)
    } else {
        Ok(Uri::builder()
            .scheme("https")
            .authority(HOST_NAME)
            .path_and_query(path)
            .build()?)
    }
}

pub fn is_syosetu_novel(uri: &Uri) -> bool {
    if let Some(host) = uri.host() {
        host == HOST_NAME
    } else {
        false
    }
}

const TITLE_SELECTOR: &'static str = ".novel_title";
const AUTHOR_SELECTOR: &'static str = "div.novel_writername > a";
const INFO_LINK_SELECTOR: &'static str = "#head_nav > li:nth-child(2) > a";
const SECTION_SELECTOR: &'static str = ".chapter_title";
const CHAPTER_SELECTOR: &'static str = ".novel_sublist2";

pub fn make_syosetu_novel(uri: Uri) -> NovelResult<Novel> {
    let node = crate::fetch_page(&uri)?;
    let mut main_page_data = TreeTraverser::new(node, MainPageData::default())
        .add_hook(TITLE_SELECTOR, None, MainPageData::get_title)?
        .add_hook(AUTHOR_SELECTOR, None, MainPageData::get_author)?
        .add_hook(INFO_LINK_SELECTOR, None, MainPageData::get_info_path)?
        .add_hook(SECTION_SELECTOR, None, MainPageData::get_section)?
        .add_hook(CHAPTER_SELECTOR, None, MainPageData::get_chapter)?
        .traverse();
    main_page_data.append_chapters_to_section();

    let title = main_page_data.title.ok_or(NovelError::ComponentMissing(NovelComponent::Title))?;
    let author = main_page_data.author.ok_or(NovelError::ComponentMissing(NovelComponent::Author))?;
    let info_path = main_page_data.info_path
        .ok_or(NovelError::ComponentMissing(NovelComponent::InfoPath))?;
    let status = info_page::fetch_status_in_info(make_uri(&info_path)?)?;
    let contents = {
        if main_page_data.sections.is_empty() {
            if main_page_data.chapters.is_empty() {
                return Err(NovelError::ComponentMissing(NovelComponent::Chapter));
            }
            let chapters = fetch_chapters(main_page_data.chapters)?;
            NovelContents::Chapters(chapters)
        } else {
            for section in main_page_data.sections.iter() {
                if section.chapters.is_empty() {
                    return Err(NovelError::ComponentMissing(NovelComponent::ChapterUnderSection));
                }
            }
            let sections = fetch_sections(main_page_data.sections)?;
            NovelContents::Sections(sections)
        }
    };

    Ok(Novel {
        title,
        author,
        status,
        source_url: uri.to_string(),
        contents,
    })
}

#[derive(Debug, Default)]
struct MainPageData {
    title: Option<String>,
    author: Option<String>,
    info_path: Option<String>,
    sections: Vec<SectionInfo>,
    chapters: Vec<ChapterInfo>,
    chapter_count: u32,
}
impl MainPageData {
    fn append_chapters_to_section(&mut self) {
        if let Some(section) = self.sections.last_mut() {
            section.chapters.append(&mut self.chapters);
        }
    }
    fn increment_and_get_chapter_count(&mut self) -> u32 {
        self.chapter_count += 1;
        self.chapter_count
    }

    fn get_title(&mut self, element: &NodeDataRef<ElementData>) {
        self.title = Some(element.text_contents());
    }
    fn get_author(&mut self, element: &NodeDataRef<ElementData>) {
        self.author = Some(element.text_contents());
    }
    fn get_info_path(&mut self, element: &NodeDataRef<ElementData>) {
        if &element.text_contents() == "小説情報" {
            let attributes = element.attributes.borrow();
            if let Some(href) = attributes.get("href") {
                self.info_path = Some(href.to_string());
            }
        }
    }

    fn get_section(&mut self, element: &NodeDataRef<ElementData>) {
        self.append_chapters_to_section();
        self.sections.push(SectionInfo {
            name: element.text_contents(),
            chapters: Vec::new(),
        });
    }
    fn get_chapter(&mut self, element: &NodeDataRef<ElementData>) {
        let node = element.as_node();
        let name_node = node.select_first(".subtitle > a").unwrap();
        let date_node = node.select_first(".long_update").unwrap();

        let mut date_children = date_node.as_node().children();
        let uploaded_date = {
            let uploaded_date_node = date_children.next().unwrap().into_text_ref().unwrap();
            let borrowed = uploaded_date_node.borrow();
            borrowed.to_string()
        };
        let date = {
            if let Some(edited_node) = date_children.next() {
                let edited_node = edited_node.into_element_ref().unwrap();
                let attributes = edited_node.attributes.borrow();
                let edit_date = attributes.get("title").unwrap().to_string();
                format!("{}（{}）", uploaded_date, edit_date)
            } else {
                uploaded_date
            }
        };

        let order_num = self.increment_and_get_chapter_count();
        let content_path = {
            let attributes = name_node.attributes.borrow();
            attributes.get("href").unwrap().to_string()
        };
        self.chapters.push(ChapterInfo {
            name: name_node.text_contents(),
            date,
            order_num,
            content_path,
        });
    }
}
#[derive(Debug, Default)]
struct SectionInfo {
    name: String,
    chapters: Vec<ChapterInfo>,
}
impl SectionInfo {
    fn fetch(self) -> NovelResult<Section> {
        let chapters = fetch_chapters(self.chapters)?;
        Ok(Section {
            name: self.name,
            chapters,
        })
    }
}
#[derive(Debug, Default)]
struct ChapterInfo {
    name: String,
    date: String,
    order_num: u32,
    content_path: String,
}
impl ChapterInfo {
    fn fetch(self) -> NovelResult<Chapter> {
        let content = content::fetch_page_content(make_uri(&self.content_path)?)?;
        Ok(Chapter {
            name: self.name,
            date: self.date,
            order_num: self.order_num,
            content,
        })
    }
}
// NOTE This needs to take a long time since they start cutting us off
fn fetch_sections(section_infos: Vec<SectionInfo>) -> NovelResult< Vec<Section> > {
    let results: Vec<_> = section_infos.into_iter()
        .map(|section| section.fetch())
        .collect();
    let mut sections = Vec::new();
    for result in results {
        sections.push(result?);
    }
    Ok(sections)
}
fn fetch_chapters(chapter_infos: Vec<ChapterInfo>) -> NovelResult< Vec<Chapter> > {
    let results: Vec<_> = chapter_infos.into_iter()
        .map(|chapter| chapter.fetch())
        .collect();
    let mut chapters = Vec::new();
    for result in results {
        chapters.push(result?);
    }
    Ok(chapters)
}
