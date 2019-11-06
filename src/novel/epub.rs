use ebook_builder::xml_tree::xhtml_prelude::*;

pub const NOVEL_CSS: &'static str = r#"\
body {
    font-family: serif-ja, serif;
}
h1, h2, h3, h4, div, p, ol, ul, li {
	margin: 0;
	padding: 0;
}
#novel_chapter {
	writing-mode: vertical-rl;
	-webkit-writing-mode: vertical-rl;
	-epub-writing-mode: vertical-rl;
}
#novel_chapter_contents {
	line-height: 1.8;
}
"#;
pub const NOVEL_CSS_NAME: &'static str = "novel.css";

// Starts the XHTML tree with the <head> completely filled out
pub fn start_xhtml(head_title: &str, body: BodyTag) -> HtmlTag {
    HtmlTag::new()
        .default_ns("http://www.w3.org/1999/xhtml")
        .ns("epub", "http://www.idpf.org/2007/ops")
        .attr_lang("ja")
        .append_child(HeadTag::new()
            .append_child(TitleTag::new().text(head_title))
            .append_child(MetaTag::new().attr_charset("UTF-8"))
            .append_child(LinkTag::new()
                .attr_rel("stylesheet")
                .attr_href("../resources/novel.css")
                .attr_type("text/css")
            )
        )
        .append_child(body)
}
