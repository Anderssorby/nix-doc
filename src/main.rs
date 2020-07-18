//! A nix package documentation search program
mod threadpool;

use crate::threadpool::ThreadPool;

use colorful::Colorful;
use regex::Regex;
use rnix::types::{AttrSet, EntryHolder, Ident, Lambda, TokenWrapper, TypedNode};
use rnix::SyntaxKind::*;
use rnix::{NodeOrToken, SyntaxNode, WalkEvent, AST};
use walkdir::WalkDir;

use std::env;
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use std::{fmt::Display, str};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Only search files which have lib in their names
const SEARCH_FILES_PAT: &str = "lib";
const DOC_INDENT: usize = 3;

struct SearchResult {
    /// Name of the function
    identifier: String,

    /// Dedented documentation comments
    doc: String,

    /// Start of the definition of the function
    defined_at_start: usize,
}

fn find_line(file: &str, pos: usize) -> usize {
    file[..pos].lines().count()
}

impl SearchResult {
    fn format<P: Display>(&self, filename: P, file: &str) -> String {
        format!(
            "{}\n{}  {}:{}\n",
            indented(&self.doc, DOC_INDENT),
            self.identifier.as_str().white().bold(),
            filename,
            find_line(file, self.defined_at_start)
        )
    }
}

/// Should the given path be searched?
fn is_searchable(fname: &Path) -> bool {
    // XXX: we should check from the base of the nixpkgs tree since the `lib` filename heuristic
    // breaks down if the entire nixpkgs is below some folder called `lib`.
    fname.to_str().map(|s| s.ends_with(".nix")).unwrap_or(false)
}

fn search_file(file: &Path, matching: &Regex) -> Result<(Vec<SearchResult>, String)> {
    let content = fs::read_to_string(file)?;
    let ast = rnix::parse(&content).as_result()?;
    Ok((search_ast(&matching, &ast), content))
}

/// Search the `dir` for files with function definitions matching `matching`
fn search<F>(dir: &Path, matching: Regex, should_search: F)
where
    F: Fn(&Path) -> bool,
{
    let pool = ThreadPool::new(4);
    let (tx, rx) = channel();

    //println!("searching {}", dir.display());
    for direntry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| should_search(e.path()) && e.path().is_file())
    {
        let my_tx = tx.clone();
        let matching = matching.clone();
        pool.push(move || {
            //println!("{}", direntry.path().display());
            let results = search_file(direntry.path(), &matching);
            if let Err(err) = results {
                eprintln!("Failure handling {}: {}", direntry.path().display(), err);
                return;
            }
            let (results, file_content) = results.unwrap();

            let formatted = results
                .iter()
                .map(|result| result.format(direntry.path().display(), &file_content))
                .collect::<Vec<_>>();
            if formatted.len() > 0 {
                my_tx
                    .send(formatted)
                    .expect("failed to send messages to display");
            }
        });
    }

    drop(tx);
    pool.done();

    while let Ok(results) = rx.recv() {
        for result in results {
            println!("{}", result);
        }
    }
}

/// Searches the given AST for functions called `identifier`
fn search_ast(identifier: &Regex, ast: &AST) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for ev in ast.node().preorder_with_tokens() {
        match ev {
            WalkEvent::Enter(enter) => {
                //println!("enter {:?}", &enter);
                if let Some(set) = enter.into_node().and_then(|elem| AttrSet::cast(elem)) {
                    results.extend(visit_attrset(identifier, &set));
                }
            }
            WalkEvent::Leave(_leave) => {
                //println!("leave {:?}", &leave);
            }
        }
    }
    results
}

/// Emits a string `s` indented by `indent` spaces
fn indented(s: &str, indent: usize) -> String {
    let indent_s = std::iter::repeat(' ').take(indent).collect::<String>();
    s.split('\n')
        .map(|line| indent_s.clone() + line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Deletes whitespace and leading comment characters
///
/// Oversight we are choosing to ignore: if you put # characters at the beginning of lines in a
/// multiline comment, they will be deleted.
fn cleanup_comments<S: AsRef<str>, I: DoubleEndedIterator<Item = S>>(comment: &mut I) -> String {
    comment
        .rev()
        .map(|comment| {
            comment
                .as_ref()
                .split("\n")
                .map(|line| {
                    line
                        // leading whitespace
                        .trim_start_matches(|c: char| c.is_whitespace() || c == '#')
                        // multiline starts
                        .trim_start_matches("/*")
                        // whitespace after multiline starts
                        .trim()
                        // whitespace after multiline ends
                        .trim_end()
                        // multiline ends
                        .trim_end_matches("*/")
                        // trailing
                        .trim_end()
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn visit_attrset(id_needle: &Regex, set: &AttrSet) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for entry in set.entries() {
        if let Some(_) = entry.value().and_then(Lambda::cast) {
            if let Some(attr) = entry.key() {
                let ident = attr.path().last().and_then(Ident::cast);
                let defined_at_start = ident
                    .as_ref()
                    .map(|i| i.node().text_range().start().to_usize());

                let ident_name = ident.as_ref().map(|id| id.as_str());

                if ident_name.map(|id| id_needle.is_match(id)) != Some(true) {
                    // rejected, not matching our pattern
                    continue;
                }

                let ident_name = ident_name.unwrap();

                if let Some(comment) = find_comment(attr.node().clone()) {
                    results.push(SearchResult {
                        identifier: ident_name.to_string(),
                        doc: comment,
                        defined_at_start: defined_at_start.unwrap(),
                    });
                } else {
                    // ignore results without comments, they are probably reexports or
                    // modifications
                    continue;
                }
            }
        }
    }
    results
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let re_match = args.next();
    let file = args.next().unwrap_or(".".to_string());
    if re_match.is_none() {
        eprintln!("Usage: list-fns <file>");
        return Ok(());
    }

    let re_match = re_match.unwrap();
    let re_match = Regex::new(&re_match)?;
    search(&Path::new(&file), re_match, is_searchable);
    Ok(())
}

fn find_comment(node: SyntaxNode) -> Option<String> {
    let mut node = NodeOrToken::Node(node);
    let mut comments = Vec::new();
    loop {
        loop {
            if let Some(new) = node.prev_sibling_or_token() {
                node = new;
                break;
            } else {
                node = NodeOrToken::Node(node.parent()?);
            }
        }

        match node.kind() {
            TOKEN_COMMENT => match &node {
                NodeOrToken::Token(token) => comments.push(token.text().clone()),
                NodeOrToken::Node(_) => unreachable!(),
            },
            t if t.is_trivia() => (),
            _ => break,
        }
    }
    let doc = cleanup_comments(&mut comments.iter().map(|c| c.as_str()));
    return Some(doc).filter(|it| !it.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_stripping() {
        let ex1 = ["/* blah blah blah\n      foooo baaar\n */"];
        assert_eq!(
            cleanup_comments(&mut ex1.iter()),
            "blah blah blah\nfoooo baaar\n"
        );

        let ex2 = ["# a1", "#    a2", "# aa"];
        assert_eq!(cleanup_comments(&mut ex2.iter()), "aa\na2\na1");
    }
}
