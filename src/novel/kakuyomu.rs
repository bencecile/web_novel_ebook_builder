mod content;

use kuchiki::{ElementData, NodeDataRef};
use isahc::http::{Uri};
use rayon::prelude::*;

use crate::{
    NovelError, NovelResult, NovelComponent,
    novel::{
        Novel, Section, Chapter, NovelStatus, NovelContents,
        novel_utils,
    },
    traverser::{TreeTraverser},
};

const HOST_NAME: &'static str = "kakuyomu.jp";
fn make_uri(path: &str) -> NovelResult<Uri> {
    Ok(Uri::builder()
        .scheme("https")
        .authority(HOST_NAME)
        .path_and_query(path)
        .build()?)
}

pub fn is_kakuyomu_novel(uri: &Uri) -> bool {
    if let Some(host_str) = uri.host() {
        if host_str != HOST_NAME {
            false
        } else  {
            uri.path().starts_with("/works/")
        }
    } else {
        // We must have a host
        false
    }
}

const TITLE_SELECTOR: &'static str = "#workTitle > a";
const AUTHOR_SELECTOR: &'static str = "#workAuthor-activityName > a";
const STATUS_SELECTOR: &'static str = "div#workInformationList > dl > dd:nth-child(2)";
const SECTION_SELECTOR: &'static str = "li.widget-toc-chapter > span";
const CHAPTER_SELECTOR: &'static str = "li.widget-toc-episode > a";
const CHAPTER_NAME_SELECTOR: &'static str = "span.widget-toc-episode-titleLabel";
const CHAPTER_DATE_SELECTOR: &'static str = "time.widget-toc-episode-datePublished";

pub fn make_kakuyomu_novel(uri: Uri) -> NovelResult<Novel> {
    let node = crate::fetch_page(&uri)?;
    let mut main_page_data = TreeTraverser::new(node, MainPageData::default())
        .add_hook(TITLE_SELECTOR, None, MainPageData::get_title)?
        .add_hook(AUTHOR_SELECTOR, None, MainPageData::get_author)?
        .add_hook(STATUS_SELECTOR, None, MainPageData::get_status)?
        .add_hook(SECTION_SELECTOR, None, MainPageData::get_section)?
        .add_hook(CHAPTER_SELECTOR, None, MainPageData::get_chapter)?
        .traverse();
    // Since we won't encounter another section (if there were any) to move the chapters
    main_page_data.move_chapters_to_section();

    let title = main_page_data.title.ok_or(NovelError::ComponentMissing(NovelComponent::Title))?;
    let author = main_page_data.author.ok_or(NovelError::ComponentMissing(NovelComponent::Author))?;
    let status = main_page_data.status.ok_or(NovelError::ComponentMissing(NovelComponent::Status))?;
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
    status: Option<NovelStatus>,
    sections: Vec<SectionInfo>,
    chapters: Vec<ChapterInfo>,
    chapter_count: u32,
}
impl MainPageData {
    fn move_chapters_to_section(&mut self) {
        if let Some(section) = self.sections.last_mut() {
            section.chapters.append(&mut self.chapters);
        }
    }
    fn increment_and_get_chapters(&mut self) -> u32 {
        self.chapter_count += 1;
        self.chapter_count
    }

    fn get_title(&mut self, element: &NodeDataRef<ElementData>) {
        self.title = Some(element.text_contents());
    }
    fn get_author(&mut self, element: &NodeDataRef<ElementData>) {
        self.author = Some(element.text_contents());
    }
    fn get_status(&mut self, element: &NodeDataRef<ElementData>) {
        let status_text = element.text_contents();
        self.status = Some(match status_text.as_str() {
            "連載中" => NovelStatus::Running,
            "完結済" => NovelStatus::Finished,
            _ => return,
        });
    }

    fn get_section(&mut self, element: &NodeDataRef<ElementData>) {
        self.move_chapters_to_section();
        self.sections.push(SectionInfo {
            name: element.text_contents(),
            chapters: Vec::new(),
        });
    }
    fn get_chapter(&mut self, element: &NodeDataRef<ElementData>) {
        let chapter_node = element.as_node();
        let name = chapter_node.select_first(CHAPTER_NAME_SELECTOR).unwrap()
            .text_contents();
        let date = chapter_node.select_first(CHAPTER_DATE_SELECTOR).unwrap()
            .text_contents();
        let date = novel_utils::convert_num_string_to_ja(&date);
        let attributes = element.attributes.borrow();
        let uri_path = attributes.get("href").unwrap().to_string();
        let order_num = self.increment_and_get_chapters();
        self.chapters.push(ChapterInfo {
            name,
            date,
            order_num,
            uri_path,
        });
    }
}

#[derive(Debug, Default)]
struct SectionInfo {
    name: String,
    chapters: Vec<ChapterInfo>,
}
impl SectionInfo {
    fn fetch_section(self) -> NovelResult<Section> {
        let chapters = fetch_chapters(self.chapters)?;
        Ok(Section {
            name: self.name,
            chapters,
        })
    }
}
fn fetch_sections(section_infos: Vec<SectionInfo>) -> NovelResult< Vec<Section> > {
    let section_results: Vec<_> = section_infos.into_par_iter()
        .map(|section| section.fetch_section())
        .collect();
    let mut sections = Vec::new();
    for section in section_results {
        sections.push(section?);
    }
    Ok(sections)
}
#[derive(Debug, Default)]
struct ChapterInfo {
    name: String,
    date: String,
    order_num: u32,
    uri_path: String,
}
impl ChapterInfo {
    fn fetch_chapter(self) -> NovelResult<Chapter> {
        let uri = make_uri(&self.uri_path)?;
        let content = content::fetch_novel_content(uri)?;
        Ok(Chapter {
            name: self.name,
            date: self.date,
            order_num: self.order_num,
            content,
        })
    }
}
fn fetch_chapters(chapter_infos: Vec<ChapterInfo>) -> NovelResult< Vec<Chapter> > {
    let fetch_results: Vec<_> = chapter_infos.into_par_iter()
        .map(|chapter| chapter.fetch_chapter())
        .collect();
    let mut chapters = Vec::new();
    for fetch_result in fetch_results {
        chapters.push(fetch_result?);
    }
    Ok(chapters)
}

// mod css_selector {
//     pub const TITLE_LINK: &'static str = "#workTitle > a";
//     pub const AUTHOR_LINK: &'static str = "#workAuthor-activityName > a";
//     pub const STATUS_INFO_NODES: &'static str = "#workInformationList > dl > dd";

//     pub const TOC_ITEMS: &'static str = ".widget-toc-items";
//     pub const SECTION_ITEM: &'static str = ".widget-toc-chapter";
//     pub const CHAPTER_ITEM: &'static str = ".widget-toc-episode";
//     pub fn sections_and_chapters() -> String { format!("{}, {}", SECTION_ITEM, CHAPTER_ITEM) }

//     pub const SECTION_NAME: &'static str = "span";
//     pub const CHAPTER_LINK: &'static str = ".widget-toc-episode-episodeTitle";
//     pub const CHAPTER_NAME: &'static str = ".widget-toc-episode-titleLabel";
//     pub const CHAPTER_DATE: &'static str = ".widget-toc-episode-datePublished";

//     pub const NOVEL_CONTENTS: &'static str = ".widget-episodeBody";
//     pub const BLANK_LINE: &'static str = "p.blank";
//     pub const RUBY: &'static str = "ruby";
//     pub const RB: &'static str = "rb";
//     pub const RT: &'static str = "rt";
// }

// struct ChapterFetchInfo {
//     uri: Uri,
//     order_num: u32,
// }
// impl ChapterFetchInfo {
//     fn new(chapter_node: &NodeRef, order_num: u32) -> NovelResult<ChapterFetchInfo> {
//         Ok(ChapterFetchInfo {
//             uri: find_chapter_url(chapter_node)?,
//             order_num,
//         })
//     }

//     fn fetch_content(self) -> NovelResult<(Vec<ContentLine>, u32)> {
//         let page_node = crate::fetch_web_page(&self.url)?;
//         let content_node = find_node(&page_node, self::css_selector::NOVEL_CONTENTS,
//             NovelComponent::ChapterContent)?;

//         let blank_line_selector = make_selector(self::css_selector::BLANK_LINE)?;
//         let content_selector = make_selector("p")?;

//         let mut content_lines = Vec::new();
//         for paragraph in content_selector.filter(content_node.as_node().children()) {
// println!("{:?}", paragraph.data());
//             let element = paragraph.clone().into_element_ref().unwrap();
//             if blank_line_selector.matches(&element) {
//                 content_lines.push(ContentLine::EmptyLine);
//             } else {
//                 content_lines.push(ContentLine::Line(
//                     find_contents_in_paragraph(&paragraph, self.url.to_string())?
//                 ));
//             }
//         }
//         Ok( (content_lines, self.order_num) )
//     }
// }
// fn fetch_contents(chapter_fetch_infos: Vec<ChapterFetchInfo>)
// -> Vec<NovelResult<(Vec<ContentLine>, u32)>> {
//     chapter_fetch_infos.into_iter()
//         .map(|info| info.fetch_content())
//         .collect()
// }
// fn find_contents_in_paragraph(paragraph_node: &NodeRef, url: String)
// -> NovelResult<Vec<Content>> {
//     let ruby_selector = make_selector(self::css_selector::RUBY)?;
//     let rb_selector = make_selector(self::css_selector::RB)?;
//     let rt_selector = make_selector(self::css_selector::RT)?;

//     let mut contents = Vec::new();
//     for p_child in paragraph_node.children() {
//         match p_child.data() {
//             NodeData::Text(text) => contents.push(Content::Span(text.borrow().clone())),
//             NodeData::Element(_) => {
//                 let element = p_child.clone().into_element_ref().unwrap();
//                 if ruby_selector.matches(&element) {
//                     let mut main: Option<String> = None;
//                     let mut above: Option<String> = None;
//                     for ruby_child in p_child.children() {
//                         match ruby_child.clone().data() {
//                             NodeData::Text(text) => {
//                                 main = Some(text.borrow().clone());
//                                 continue;
//                             },
//                             NodeData::Element(_) => {
//                                 let element = ruby_child.clone().into_element_ref().unwrap();
//                                 if rb_selector.matches(&element) {
//                                     main = Some(ruby_child.text_contents());
//                                 } else if rt_selector.matches(&element) {
//                                     above = Some(ruby_child.text_contents());
//                                 }
//                             }
//                             _ => (),
//                         }

//                         if main.is_some() && above.is_some() {
//                             contents.push(Content::Ruby {
//                                 main: main.take().unwrap(),
//                                 above: above.take().unwrap(),
//                             });
//                         }
//                     }
//                     if main.is_some() || above.is_some() {
//                         return Err(NovelError::MalformedHtml(url, "Bad ruby".to_string()));
//                     }
//                 }
//             },
//             _ => (),
//         }
//     }
//     Ok(contents)
// }

// fn find_title(node: &NodeRef) -> NovelResult<String> {
//     find_text(node, self::css_selector::TITLE_LINK, NovelComponent::Title)
// }
// fn find_author(node: &NodeRef) -> NovelResult<String> {
//     find_text(node, self::css_selector::AUTHOR_LINK, NovelComponent::Author)
// }
// fn find_status(node: &NodeRef) -> NovelResult<NovelStatus> {
//     let status_text = {
//         let selector = self::css_selector::STATUS_INFO_NODES;
//         let mut info_nodes = node.select(selector)
//             .map_err(|_| NovelError::BadCssSelector(selector.to_string()))?;
//         if let Some(status_node) = info_nodes.next() {
//             status_node.text_contents()
//         } else {
//             return Err(NovelError::ComponentMissing(NovelComponent::Status));
//         }
//     };
//     match status_text.as_str() {
//         "連載中" => Ok(NovelStatus::Running),
//         "完結済" => Ok(NovelStatus::Finished),
//         _ => Err(NovelError::ComponentMissing(NovelComponent::Status)),
//     }
// }

// fn find_toc(node: &NodeRef) -> NovelResult<NodeDataRef<ElementData>> {
//     find_node(node, self::css_selector::TOC_ITEMS, NovelComponent::Chapter)
// }
// fn has_sections(toc_node: &NodeRef) -> bool {
//     toc_node.select_first(self::css_selector::SECTION_ITEM).is_ok()
// }
// fn find_sections_and_chapters(toc_node: &NodeRef) -> NovelResult<NovelContents> {
//     let mut sections: Vec<Section> = Vec::new();
//     let toc_selector = self::css_selector::sections_and_chapters();
//     let toc_nodes = toc_node.select(&toc_selector)
//         .map_err(|_| NovelError::BadCssSelector(toc_selector))?;

//     let section_selector = make_selector(self::css_selector::SECTION_ITEM)?;

//     let mut chapter_fetch_infos: Vec<ChapterFetchInfo> = Vec::new();
//     let mut chapter_count: u32 = 0;
//     for section_or_chapter in toc_nodes {
//         if section_selector.matches(&section_or_chapter) {
//             let name = find_text(section_or_chapter.as_node(),
//                 self::css_selector::SECTION_NAME,
//                 NovelComponent::Section
//             )?;
//             sections.push(Section {
//                 name,
//                 chapters: Vec::new(),
//             });
//         } else {
//             chapter_count += 1;
//             let chapter_node = section_or_chapter.as_node();
//             let section = sections.last_mut()
//                 .ok_or(NovelError::ComponentMissing(NovelComponent::Section))?;
//             let (chapter, chapter_fetch_info) = make_empty_chapter(chapter_node, chapter_count)?;
//             section.chapters.push(chapter);
//             chapter_fetch_infos.push(chapter_fetch_info);
//         }
//     }
//     for section in sections.iter() {
//         if section.chapters.is_empty() {
//             return Err(NovelError::ComponentMissing(NovelComponent::Chapter));
//         }
//     }

//     for chapter_result in fetch_contents(chapter_fetch_infos) {
//         let (mut content_lines, order_num) = chapter_result?;
//         let chapter = sections.iter_mut()
//             .find_map(|section| section.chapters.iter_mut()
//                 .find(|chapter| chapter.order_num == order_num)
//             ).expect(&format!("Failed to find the {} chapter", order_num));
//         chapter.content.append(&mut content_lines);
//     }
//     Ok(NovelContents::Sections(sections))
// }
// fn find_chapters(toc_node: &NodeRef) -> NovelResult<NovelContents> {
//     let mut chapters = Vec::new();
//     let mut chapter_fetch_infos = Vec::new();
//     let chapter_nodes = toc_node.select(self::css_selector::CHAPTER_ITEM)
//         .map_err(|_| NovelError::BadCssSelector(self::css_selector::CHAPTER_ITEM.to_string()))?;

//     for (i, chapter_node) in chapter_nodes.enumerate() {
//         let (chapter, chapter_fetch_info) = make_empty_chapter(
//             chapter_node.as_node(), (i + 1) as u32
//         )?;
//         chapters.push(chapter);
//         chapter_fetch_infos.push(chapter_fetch_info);
//     }

//     for chapter_result in fetch_contents(chapter_fetch_infos) {
//         let (mut content_lines, order_num) = chapter_result?;
//         let chapter = chapters.iter_mut()
//             .find(|chapter| chapter.order_num == order_num)
//             .expect(&format!("Failed to find the {} chapter", order_num));
//         chapter.content.append(&mut content_lines);
//     }
//     Ok(NovelContents::Chapters(chapters))
// }
// fn make_empty_chapter(chapter_node: &NodeRef, order_num: u32)
// -> NovelResult<(Chapter, ChapterFetchInfo)> {
//     // Leave the content empty until we can fetch it
//     let chapter = Chapter {
//         name: find_chapter_name(chapter_node)?,
//         date: find_chapter_date(chapter_node)?,
//         order_num,
//         content: Vec::new(),
//     };
//     let chapter_fetch_info = ChapterFetchInfo::new(chapter_node, order_num)?;
//     Ok( (chapter, chapter_fetch_info) )
// }
// fn find_chapter_name(chapter_node: &NodeRef) -> NovelResult<String> {
//     find_text(chapter_node, self::css_selector::CHAPTER_NAME, NovelComponent::ChapterName)
// }
// fn find_chapter_date(chapter_node: &NodeRef) -> NovelResult<String> {
//     find_text(chapter_node, self::css_selector::CHAPTER_DATE, NovelComponent::ChapterDate)
// }
// fn find_chapter_url(chapter_node: &NodeRef) -> NovelResult<Uri> {
//     let chapter_link = find_node(chapter_node, self::css_selector::CHAPTER_LINK,
//         NovelComponent::Chapter)?;
//     let attributes = chapter_link.attributes.borrow();
//     let href = attributes.get("href")
//         .ok_or(NovelError::ComponentMissing(NovelComponent::Chapter))?;
//     make_uri(href)
// }

// fn find_node(node: &NodeRef, selector: &str, component: NovelComponent)
// -> NovelResult<NodeDataRef<ElementData>> {
//     node.select_first(selector)
//         .map_err(|_| NovelError::ComponentMissing(component))
// }
// fn find_text(node: &NodeRef, selector: &str, component: NovelComponent)
// -> NovelResult<String> {
//     let selected_node = find_node(node, selector, component)?;
//     Ok(selected_node.text_contents())
// }
// fn make_selector(selector: &str) -> NovelResult<Selectors> {
//     Selectors::compile(selector)
//         .map_err(|_| NovelError::BadCssSelector(selector.to_string()))
// }
