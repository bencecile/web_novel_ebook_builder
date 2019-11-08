mod novel;
mod traverser;

use std::{
    fs,
    io::{Error as IOError},
    path::{PathBuf},
    time::{Instant},
};
use isahc::{
    Error as IsahcError,
    http::{
        Error as HttpError, Uri,
        uri::InvalidUri,
    },
    prelude::*,
};
use kuchiki::{
    NodeRef,
    traits::*,
};
use rayon::{ThreadPoolBuilder};
use serde::{Deserialize};

use ebook_builder::{
    BookError,
    xml_tree::{XmlError},
};

use crate::{
    novel::{Novel, NovelSite},
    traverser::{TraverseError},
};

#[derive(Deserialize)]
struct RunInfo {
    save_dir: PathBuf,
    novels: Vec<NovelInfo>,
}
#[derive(Deserialize)]
struct NovelInfo {
    url: String,
    short_name: String,
}

fn main() {
    // We will want a lot of extra threads since we will be waiting on IO
    ThreadPoolBuilder::new().num_threads(20).build_global()
        .expect("Failed to set the global thread pool");

    let run_info: RunInfo = toml::from_str(
        &fs::read_to_string("novel_info.toml").expect("Failed to read the info file")
    ).expect("Failed to convert the info file");

    for novel_info in run_info.novels {
        println!("Starting {}", &novel_info.short_name);
        let start = Instant::now();

        let novel = match fetch_novel(&novel_info.url) {
            Ok(novel) => novel,
            Err(e) => {
                println!("Failed {}: {:?}", &novel_info.short_name, e);
                continue;
            },
        };
        match novel.save_epubs(&run_info.save_dir) {
            Err(e) => {
                println!("Failed to save {} ({}): {:?}",
                    novel.print_name(), &novel_info.short_name, e);
                continue;
            },
            _ => (),
        }
        println!("Finished {} ({}) in {:?}",
            novel.print_name(), &novel_info.short_name, start.elapsed());
    }
}

pub type NovelResult<T> = Result<T, NovelError>;
#[derive(Debug)]
pub enum NovelError {
    NotANovel,
    ComponentMissing(NovelComponent),

    BookError(BookError),
    HttpError(HttpError),
    InvalidUri(InvalidUri),
    IOError(IOError),
    IsahcError(IsahcError),
    TraverseError(TraverseError),
    XmlError(XmlError),
}
impl From<BookError> for NovelError {
    fn from(error: BookError) -> Self { Self::BookError(error) }
}
impl From<HttpError> for NovelError {
    fn from(error: HttpError) -> Self { Self::HttpError(error) }
}
impl From<InvalidUri> for NovelError {
    fn from(error: InvalidUri) -> Self { Self::InvalidUri(error) }
}
impl From<IOError> for NovelError {
    fn from(error: IOError) -> Self { Self::IOError(error) }
}
impl From<IsahcError> for NovelError {
    fn from(error: IsahcError) -> Self { Self::IsahcError(error) }
}
impl From<TraverseError> for NovelError {
    fn from(error: TraverseError) -> Self { Self::TraverseError(error) }
}
impl From<XmlError> for NovelError {
    fn from(error: XmlError) -> Self { Self::XmlError(error) }
}

#[derive(Debug, Copy, Clone)]
pub enum NovelComponent {
    Title,
    Author,
    Date,
    Status,
    Chapter,
    ChapterContent,
    ChapterUnderSection,
}

fn fetch_novel(novel_url: &str) -> NovelResult<Novel> {
    let uri: Uri = novel_url.parse()?;
    let novel_site = NovelSite::is_a_novel(&uri)
        .ok_or(NovelError::NotANovel)?;
    novel_site.make_novel(uri)
}

fn fetch_page(uri: &Uri) -> NovelResult<NodeRef> {
    let page_text = isahc::get(uri)?.text()?;
    Ok(kuchiki::parse_html().one(page_text))
}

fn sanitize_book_name(book_name: &str) -> String {
    book_name.chars().map(|c| match c {
        '?' => '？',
        '/' => '／',
        '\\' => '＼',
        ':' => '：',
        _ => c,
    }).collect()
}
