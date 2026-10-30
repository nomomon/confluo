use super::parse;
use crate::ui::molecules::markdown::blocks::{Alert, Align, Block};
use crate::ui::molecules::markdown::html::preprocess;

fn md(src: &str) -> Vec<Block> {
    parse(&preprocess(src))
}

#[test]
fn table_links_images_and_alignment() {
    let src = "| Package | Age |\n|:---|---:|\n| [react](https://x.dev) | ![age](https://img/a.svg) |\n";
    let blocks = md(src);
    let Some(Block::Table { aligns, head, rows }) = blocks.first() else { panic!("expected a table") };
    assert_eq!(aligns, &[Align::Left, Align::Right]);
    assert_eq!(head.len(), 2);
    assert_eq!(rows[0][0].text, "react");
    assert_eq!(rows[0][0].links()[0].1, "https://x.dev");
    assert_eq!(rows[0][1].text, "age");
    assert_eq!(rows[0][1].images().collect::<Vec<_>>(), ["https://img/a.svg"]);
}

#[test]
fn tight_task_list_and_nesting() {
    let blocks = md("- [x] done\n- [ ] todo\n  1. inner\n  2. two\n");
    let Some(Block::List { items, .. }) = blocks.first() else { panic!("expected a list") };
    assert_eq!(items[0].task, Some(true));
    assert_eq!(items[1].task, Some(false));
    assert!(matches!(items[0].blocks[0], Block::Paragraph(_)));
    assert!(matches!(items[1].blocks[1], Block::List { start: Some(1), .. }));
}

#[test]
fn gfm_alert() {
    let blocks = md("> [!WARNING]\n> Careful\n");
    assert!(matches!(blocks.first(), Some(Block::Quote { alert: Some(Alert::Warning), .. })));
}

#[test]
fn fenced_code_keeps_language() {
    let blocks = md("```rust\nfn main() {}\n```\n");
    let Some(Block::Code { lang, code }) = blocks.first() else { panic!("expected code") };
    assert_eq!((lang.as_str(), code.as_str()), ("rust", "fn main() {}"));
}

#[test]
fn details_fold_markdown_body() {
    let blocks = md("<details>\n<summary>Release notes</summary>\n\n- one\n- two\n\n</details>\n\nafter");
    let Some(Block::Details { summary, blocks: inner }) = blocks.first() else { panic!("expected details") };
    assert_eq!(summary, "Release notes");
    assert!(matches!(inner.first(), Some(Block::List { .. })));
    assert!(matches!(blocks.get(1), Some(Block::Paragraph(_))));
}

#[test]
fn unclosed_details_and_stray_close() {
    let blocks = md("</details>\n\n<details><summary>S</summary>\n\nbody");
    assert!(matches!(blocks.first(), Some(Block::Details { .. })));
}

#[test]
fn autolinks_and_strikethrough() {
    let blocks = md("See https://example.com/a. and ~~old~~ <https://b.dev>");
    let Some(Block::Paragraph(p)) = blocks.first() else { panic!() };
    let links: Vec<String> = p.links().into_iter().map(|l| l.1).collect();
    assert_eq!(links, ["https://example.com/a", "https://b.dev"]);
    assert!(p.runs.iter().any(|(_, s)| s.strike));
}

#[test]
fn footnotes_minimal() {
    let blocks = md("Text[^1]\n\n[^1]: The note.\n");
    let Some(Block::Paragraph(p)) = blocks.first() else { panic!() };
    assert!(p.text.contains("[1]"));
    assert!(blocks.iter().any(|b| matches!(b, Block::Footnote { label, .. } if label == "1")));
}

#[test]
fn image_without_alt_still_shows() {
    let blocks = md("![](https://x/y.png)");
    let Some(Block::Paragraph(p)) = blocks.first() else { panic!() };
    assert_eq!(p.images().count(), 1);
}
