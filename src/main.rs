mod novel;

use std::{
    io::{Error as IOError},
    time::{Instant},
};
use kuchiki::{
    traits::*,
};
use reqwest::{Error as ReqError, Url, UrlError};

use ebook_builder::{
    BookError,
    xml_tree::{XmlError},
};

use crate::{
    novel::{Novel, NovelSite},
};

const SAVE_DIR: &'static str = r"C:\Manga\!BooksToCopy";
const NOVELS: &[&'static str] = &[
    "https://kakuyomu.jp/works/1177354054881165840", // Shinchou Yuusha
];

fn main() {
    for &novel_url in NOVELS.iter() {
        println!("Starting {}\r", novel_url);
        let start = Instant::now();

        let novel = match fetch_novel(novel_url) {
            Ok(novel) => novel,
            Err(e) => {
                println!("{} failed: {:?}", novel_url, e);
                continue;
            },
        };
        match novel.save_epubs(SAVE_DIR) {
            Err(e) => println!("Failed to save {} ({}): {:?}", novel.print_name(), novel_url, e),
            _ => (),
        }
        println!("Finished {} ({}) in {:?}", novel.print_name(), novel_url, start.elapsed());
    }
}

#[derive(Debug)]
pub enum NovelError {
    NotANovelURL,
    ComponentMissing(NovelComponent),

    BadURL(UrlError),
    BookError(BookError),
    ReqwestError(ReqError),
    IOError(IOError),
    XmlError(XmlError),
}
impl From<BookError> for NovelError {
    fn from(error: BookError) -> Self { Self::BookError(error) }
}
impl From<UrlError> for NovelError {
    fn from(error: UrlError) -> Self { Self::BadURL(error) }
}
impl From<ReqError> for NovelError {
    fn from(error: ReqError) -> Self { Self::ReqwestError(error) }
}
impl From<IOError> for NovelError {
    fn from(error: IOError) -> Self { Self::IOError(error) }
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
}

fn fetch_novel(novel_site_str: &str) -> Result<Novel, NovelError> {
    let novel_url = Url::parse(novel_site_str)?;
    let novel_site = {
        if let Some(novel_site) = NovelSite::is_a_novel(&novel_url) {
            novel_site
        } else {
            return Err(NovelError::NotANovelURL);
        }
    };
    let page_text = reqwest::get(novel_url)?.text()?;
    let page_node = kuchiki::parse_html().one(page_text);
    let novel = novel_site.make_novel(novel_site_str, page_node)?;
    Ok(novel)
}
