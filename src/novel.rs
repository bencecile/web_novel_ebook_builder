mod epub;
mod kakuyomu;

use std::{
    path::{Path},
};
use kuchiki::{NodeRef};
use reqwest::{Url};

use ebook_builder::{
    Book, EBookType, FileType, ReadingDir,
    xml_tree::xhtml_prelude::*,
};

use crate::{NovelError};

#[derive(Debug)]
pub struct Novel {
    title: String,
    author: String,
    status: NovelStatus,
    source_url: String,
    // Since there may not be any sections
    contents: NovelContents,
}
impl Novel {
    pub fn print_name(&self) -> String { format!("{} [{}]", &self.title, &self.author) }
    pub fn save_epubs(&self, save_dir: impl AsRef<Path>) -> Result<(), NovelError> {
        let save_dir = save_dir.as_ref();
        match &self.contents {
            NovelContents::Sections(sections) => {
                let books = self.make_section_epubs(&sections)?;
                for (book, book_name) in books {
                    let book_path = save_dir.join(format!("{}.epub", book_name));
                    book.save_to_file(EBookType::Epub, book_path, true)?;
                }
            },
            NovelContents::Chapters(chapters) => {
                let (book, book_name) = self.make_chapter_epub(&chapters)?;
                let book_path = save_dir.join(format!("{}.epub", book_name));
                book.save_to_file(EBookType::Epub, book_path, true)?;
            },
        }
        Ok(())
    }

    fn start_book(&self) -> Result<Book, NovelError> {
        let mut book = Book::new(&self.title, ReadingDir::Rtl, "ja");
        book.add_author(&self.author, None);
        let title_page: Vec<u8> = epub::start_xhtml("表紙", BodyTag::new()
                .append_child(H1Tag::new().text(&self.title))
                .append_child(H2Tag::new().text(&self.author))
                .append_child(H3Tag::new()
                    .text("投稿版　")
                    .text(self.status.status_text())
                )
                .append_child(ATag::new()
                    .attr_href(&self.source_url)
                    // Display it as text in case the link doesn't work
                    .text(&self.source_url)
                )
            ).write_doc_to(Vec::new())?;
        book.add_file_as_bytes("title-cover.xhtml", &title_page, FileType::Xhtml);
        book.mark_as_chapter_start("表紙");

        book.add_file_as_bytes(epub::NOVEL_CSS_NAME, epub::NOVEL_CSS.as_bytes(), FileType::Css);

        Ok(book)
    }

    fn make_section_epubs(&self, sections: &[Section]) -> Result<Vec<(Book, String)>, NovelError> {
        let base_book = self.start_book()?;
        let mut books = Vec::new();

        let total_sections = sections.len();
        for (i, section) in sections.iter().enumerate() {
            let book = section.fill_out_book(i + 1, base_book.clone())?;
            let book_name = self.section_book_name(section, i, total_sections);
            books.push( (book, book_name) );
        }
        Ok(books)
    }
    fn make_chapter_epub(&self, chapters: &[Chapter]) -> Result<(Book, String), NovelError> {
        let mut book = self.start_book()?;
        for chapter in chapters.iter() {
            chapter.add_to_book(&mut book)?;
        }
        Ok( (book, self.chapters_book_name(chapters)) )
    }

    fn section_book_name(&self, section: &Section, section_index: usize, total_sections: usize)
    -> String {
        let max_sections_num_digits = total_sections.to_string().len();
        // The section number will need to be left padded with 0s
        //  So that each number will have to same number of digits
        let section_num = {
            // Starting at 0 for book numbers feels weird
            let simple_section_num = (section_index + 1).to_string();
            let diff = max_sections_num_digits - simple_section_num.len();
            if diff > 0 {
                "0".repeat(diff) + &simple_section_num
            } else {
                simple_section_num
            }
        };
        let chapter_range = chapter_range(&section.chapters);
        let kan_stamp = if section_index == total_sections - 1 {
            self.status.kan_stamp()
        } else { "" };
        format!("{} 第{}章 「{}」 [{}] (投稿版) ({}部分-{}部分){}",
            &self.title, section_num, &section.name, &self.author,
            chapter_range.0, chapter_range.1, kan_stamp)
    }
    fn chapters_book_name(&self, chapters: &[Chapter]) -> String {
        let chapter_range = chapter_range(chapters);
        format!("{} [{}] (投稿版) ({}部分-{}部分){}",
            &self.title, &self.author,
            chapter_range.0, chapter_range.1, self.status.kan_stamp())
    }
}

#[derive(Debug, Copy, Clone)]
enum NovelStatus {
    Running,
    Finished,
}
impl NovelStatus {
    fn kan_stamp(self) -> &'static str {
        match self {
            Self::Running => "",
            // The space needs to be here for easy formatting
            Self::Finished => " (完)",
        }
    }
    fn status_text(self) -> &'static str {
        match self {
            Self::Running => "連載中",
            Self::Finished => "完結済",
        }
    }
}

#[derive(Debug)]
enum NovelContents {
    Sections(Vec<Section>),
    Chapters(Vec<Chapter>),
}

#[derive(Debug)]
struct Section {
    name: String,
    chapters: Vec<Chapter>,
}
impl Section {
    fn fill_out_book(&self, section_num: usize, mut book: Book) -> Result<Book, NovelError> {
        // Make a new page that will just have the name of the section
        //  This will probably be just after the main page
        let section_cover: Vec<u8> = epub::start_xhtml("章の表紙", BodyTag::new()
                .append_child(H1Tag::new().text(&format!("第{}章", section_num)))
                .append_child(H1Tag::new().text(&self.name))
            )
            .write_doc_to(Vec::new())?;
        book.add_file_as_bytes("section-cover.xhtml", &section_cover, FileType::Xhtml);
        book.mark_as_chapter_start("章の表紙");

        for chapter in self.chapters.iter() {
            chapter.add_to_book(&mut book)?;
        }

        Ok(book)
    }
}

#[derive(Debug)]
struct Chapter {
    name: String,
    // This can be anything that we find. Don't want to parse this.
    date: String,
    order_num: u32,
    // The content MUST NOT have the name of the chapter
    //  We will insert it ourselves so that it will always show up exactly the way we want
    content: Vec<ContentLine>,
}
impl Chapter {
    fn make_xhtml(&self) -> HtmlTag {
        let content = self.content.iter()
            .fold(DivTag::new().attr_id("novel_chapter_contents"),
                |tag, content_line| tag.append_child(content_line.make_xhtml()));

        epub::start_xhtml(&self.name, BodyTag::new()
            .attr_id("novel_chapter")
            .append_child(H1Tag::new().text(&self.name))
            .append_child(H2Tag::new().text(&self.date))
            .append_child(H3Tag::new().text(
                &format!("{}部分目", convert_num_string_to_ja(&self.order_num.to_string()))
            ))
            .append_child(content)
        )
    }
    fn add_to_book(&self, book: &mut Book) -> Result<(), NovelError> {
        let chapter_page: Vec<u8> = self.make_xhtml()
            .write_doc_to(Vec::new())?;
        let chapter_file_name = format!("chapter-{}.xhtml", self.order_num);
        book.add_file_as_bytes(&chapter_file_name, &chapter_page, FileType::Xhtml);
        book.mark_as_chapter_start(&self.name);
        Ok(())
    }
}

fn convert_num_string_to_ja(num_string: &str) -> String {
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
        _ => panic!("Didn't get a number"),
    }).collect()
}

#[derive(Debug)]
enum ContentLine {
    Line(Vec<Content>),
    EmptyLine,
}
impl ContentLine {
    fn make_xhtml(&self) -> PTag {
        match self {
            Self::Line(contents) => contents.iter()
                .fold(PTag::new(), |tag, content| content.append_to(tag)),
            Self::EmptyLine => PTag::new(),
        }
    }
}
#[derive(Debug)]
enum Content {
    Span(String),
    Ruby {
        main: String,
        above: String,
    },
}
impl Content {
    fn append_to(&self, tag: PTag) -> PTag {
        match self {
            Self::Span(text) => tag.text(&text),
            Self::Ruby { main, above } => tag.append_child(RubyTag::new()
                .text(&main)
                .append_child(RpTag::new().text("（"))
                .append_child(RtTag::new().text(&above))
                .append_child(RpTag::new().text("）"))
            ),
        }
    }
}

fn chapter_range(chapters: &[Chapter]) -> (u32, u32) {
    let chapter_nums = chapters.iter().map(|chapter| chapter.order_num);
    let min_chapter_num = chapter_nums.clone().min().unwrap();
    let max_chapter_num = chapter_nums.max().unwrap();
    (min_chapter_num, max_chapter_num)
}

pub enum NovelSite {
    Kakuyomu,
}
impl NovelSite {
    pub fn is_a_novel(url: &Url) -> Option<NovelSite> {
        if self::kakuyomu::is_kakuyomu_novel(url) {
            Some(Self::Kakuyomu)
        } else {
            None
        }
    }

    // This should make as many other web requests as it needs
    pub fn make_novel(&self, novel_site: &str, page_node: NodeRef) -> Result<Novel, NovelError> {
        match self {
            Self::Kakuyomu => self::kakuyomu::make_kakuyomu_novel(novel_site, page_node),
        }
    }
}
