//! The naming stage across the set (SPEC-0001 R11; plan, Naming rules).
//!
//! This stage reads the pull requests and the commits of the set, each member that is a string
//! and raised no error, whatever the other errors of its document, so that the names are checked
//! in the same pass as every other error (ADR 0009 C3):
//!
//! - `branch_name`: the `branch` of a pull request, by the grammar of
//!   [`crate::naming::branch`];
//! - `commit_header`: the `header` of a commit and the `title` of a pull request, by the grammar
//!   of [`crate::naming::commit`]; and the body of a commit whose header marks a breaking change,
//!   which needs a `BREAKING CHANGE:` paragraph or footer (reason `requires` at `/body`). A
//!   commit whose header git wrote itself, such as `Merge ` or `Revert "`, is exempt. The
//!   evidence of a breaking pull request title is its label `breaking-change`, checked by
//!   SPEC-0021, not here.
//!
//! The errors of a document come in the byte order of their pointers, as in the schema pass:
//! `/body`, then `/branch` and `/header`, then `/title`.

use serde_json::{Map, Value};
use wayloop_schemas::contract::rules::TITLE;
use wayloop_schemas::validate::{DocumentSummary, InputKind, Policy};

use super::input::{Body, Entry};
use super::members::{self, BODY, BRANCH, HEADER};
use crate::naming::commit::{self, HeaderNames};
use crate::naming::{Names, branch, titles};
use crate::{Error, Pointer};

/// The inputs whose names are checked.
const NAMED: [InputKind; 2] = [InputKind::PullRequest, InputKind::Commit];

/// A pull request or a commit of the set, by its index in the input, with the members this
/// stage reads and its entry.
#[derive(Debug, Clone)]
pub(super) struct Named<'a> {
    document: u64,
    entry: &'a Entry,
    texts: Texts<'a>,
}

/// The members of an input that this stage reads, each `None` when it is not a string or raised
/// an error.
#[derive(Debug, Clone, Copy)]
enum Texts<'a> {
    PullRequest {
        title: Option<&'a str>,
        branch: Option<&'a str>,
    },
    Commit {
        header: Option<&'a str>,
        body: CommitBody<'a>,
    },
}

/// The body of a commit, as the rule of the breaking mark reads it.
#[derive(Debug, Clone, Copy)]
enum CommitBody<'a> {
    /// The commit has one line.
    Absent,
    Text(&'a str),
    /// The body is not a string, or raised an error: it is not read again.
    Refused,
}

impl CommitBody<'_> {
    /// Whether the body states no breaking change; `false` when it is refused.
    fn lacks_breaking_change(self) -> bool {
        match self {
            CommitBody::Absent => true,
            CommitBody::Text(text) => !commit::has_breaking_change(text),
            CommitBody::Refused => false,
        }
    }
}

/// The pull request or the commit of `entry`, the document `document` summarised by `summary`,
/// whose own errors are `refused`; `None` for any other document. `pointers` are those of
/// [`Pointers::new`], built once for the set.
pub(super) fn named<'a>(
    document: u64,
    entry: &'a Entry,
    summary: &DocumentSummary,
    refused: &[Error],
    pointers: &Pointers,
) -> Option<Named<'a>> {
    let Body::Document { document: body, .. } = &entry.body else {
        return None;
    };
    let read = |pointer: &Pointer, key: &str| clean(body, refused, pointer, key);
    let texts = match summary.input_kind? {
        InputKind::PullRequest => Texts::PullRequest {
            title: read(&pointers.title, TITLE),
            branch: read(&pointers.branch, BRANCH),
        },
        InputKind::Commit => Texts::Commit {
            header: read(&pointers.header, HEADER),
            body: if body.contains_key(BODY) {
                read(&pointers.body, BODY).map_or(CommitBody::Refused, CommitBody::Text)
            } else {
                CommitBody::Absent
            },
        },
        InputKind::HandoverInput => return None,
    };
    Some(Named {
        document,
        entry,
        texts,
    })
}

/// The text at `key` of `body`, at `pointer`, when no error of `refused` is at that pointer.
fn clean<'a>(
    body: &'a Map<String, Value>,
    refused: &[Error],
    pointer: &Pointer,
    key: &str,
) -> Option<&'a str> {
    if refused.iter().any(|error| error.pointer() == pointer) {
        return None;
    }
    members::text(body, key)
}

/// The pointer of the member `key` of an input.
fn pointer(key: &str) -> Pointer {
    Pointer::root().join(key)
}

/// The pointers of the members of the inputs that this stage reads, built once for the set.
pub(super) struct Pointers {
    title: Pointer,
    branch: Pointer,
    header: Pointer,
    body: Pointer,
}

impl Pointers {
    pub(super) fn new() -> Self {
        Self {
            title: titles::pointer(),
            branch: pointer(BRANCH),
            header: pointer(HEADER),
            body: pointer(BODY),
        }
    }
}

/// Whether the set summarised by `documents` holds a pull request or a commit, whose checks
/// read the policy (R21).
pub(super) fn reads_policy(documents: &[DocumentSummary]) -> bool {
    documents
        .iter()
        .any(|summary| summary.input_kind.is_some_and(|kind| NAMED.contains(&kind)))
}

/// Every error of the names of `named`, against the types and scopes of `policy`, indexed
/// once, in the order of `named`.
pub(super) fn errors(named: &[Named<'_>], policy: &Policy, pointers: &Pointers) -> Vec<Error> {
    let lists = HeaderNames {
        types: Names::new(&policy.types),
        scopes: Names::new(&policy.scopes),
    };
    let mut errors = Vec::new();
    for input in named {
        let own = match input.texts {
            Texts::PullRequest { title, branch } => {
                let mut own = Vec::new();
                if let Some(name) = branch {
                    own.extend(branch::errors(name, &lists.types, &pointers.branch));
                }
                if let Some(title) = title {
                    own.extend(commit::check(title, &lists, &pointers.title).errors);
                }
                own
            }
            Texts::Commit { header, body } => header
                .filter(|header| !commit::is_generated(header))
                .map_or_else(Vec::new, |header| {
                    let checked = commit::check(header, &lists, &pointers.header);
                    let unevidenced = checked.breaking && body.lacks_breaking_change();
                    let mut own = Vec::new();
                    if unevidenced {
                        own.push(commit::requires(
                            pointers.body.clone(),
                            pointers.header.clone(),
                        ));
                    }
                    own.extend(checked.errors);
                    own
                }),
        };
        errors.extend(
            own.into_iter()
                .map(|error| input.entry.locate(error.with_document(input.document))),
        );
    }
    errors
}

#[cfg(test)]
mod tests;
