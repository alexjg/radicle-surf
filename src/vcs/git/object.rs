use crate::vcs::git::error::*;
use git2;
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::str;

/// `Author` is the static information of a
/// [`git2::Signature`](https://docs.rs/git2/0.11.0/git2/struct.Signature.html).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Author {
    /// Name of the author.
    pub name: String,
    /// Email of the author.
    pub email: String,
    /// Time the action was taken, e.g. time of commit.
    pub time: git2::Time,
}

impl std::fmt::Debug for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Signature {{ name: {}, email: {} }}",
            self.name, self.email
        )
    }
}

impl<'repo> TryFrom<git2::Signature<'repo>> for Author {
    type Error = str::Utf8Error;

    fn try_from(signature: git2::Signature) -> Result<Self, Self::Error> {
        let name = str::from_utf8(signature.name_bytes())?.into();
        let email = str::from_utf8(signature.email_bytes())?.into();
        let time = signature.when();

        Ok(Author { name, email, time })
    }
}

/// `Commit` is the static information of a [`git2::Commit`](https://docs.rs/git2/0.11.0/git2/struct.Commit.html).
/// To get back the original `Commit` in the repository we can use the `git2::Oid` to retrieve it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Commit {
    /// Object ID of the Commit, i.e. the SHA1 digest.
    pub id: git2::Oid,
    /// The author of the commit.
    pub author: Author,
    /// The actor who committed this commit.
    pub committer: Author,
    /// The long form message of the commit.
    pub message: String,
    /// The summary message of the commit.
    pub summary: String,
}

impl<'repo> TryFrom<git2::Commit<'repo>> for Commit {
    type Error = str::Utf8Error;

    fn try_from(commit: git2::Commit) -> Result<Self, Self::Error> {
        let id = commit.id();
        let author = Author::try_from(commit.author())?;
        let committer = Author::try_from(commit.committer())?;
        let message_raw = commit.message_bytes();
        let message = str::from_utf8(message_raw)?.into();
        let summary_raw = commit.summary_bytes().expect("TODO");
        let summary = str::from_utf8(summary_raw)?.into();

        Ok(Commit {
            id,
            author,
            committer,
            message,
            summary,
        })
    }
}

/// A newtype wrapper over `String` to separate out
/// the fact that a caller wants to fetch a branch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BranchName(String);

impl TryFrom<&[u8]> for BranchName {
    type Error = str::Utf8Error;

    fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(name).map(|name| Self(String::from(name)))
    }
}

impl BranchName {
    /// Create a new `BranchName`.
    pub fn new(name: &str) -> Self {
        BranchName(name.into())
    }

    /// Access the string value of the `BranchName`.
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// The static information of a `git2::Branch`.
///
/// **Note**: The `PartialOrd` and `Ord` implementations compare on `BranchName`
/// only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// Name identifier of the `Branch`.
    pub name: BranchName,
    /// Whether the `Branch` is `Remote` or `Local`.
    pub locality: git2::BranchType,
}

impl PartialOrd for Branch {
    fn partial_cmp(&self, other: &Branch) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Branch {
    fn cmp(&self, other: &Branch) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Branch {
    /// Helper to create a remote `Branch` with a name
    pub fn remote(name: BranchName) -> Self {
        Branch {
            name,
            locality: git2::BranchType::Remote,
        }
    }

    /// Helper to create a remote `Branch` with a name
    pub fn local(name: BranchName) -> Self {
        Branch {
            name,
            locality: git2::BranchType::Local,
        }
    }
}

impl<'repo> TryFrom<git2::Reference<'repo>> for Branch {
    type Error = Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        if !reference.is_branch() {
            return Err(Error::NotBranch);
        }

        let name = BranchName::try_from(reference.name_bytes())?;
        let locality = if reference.is_remote() {
            git2::BranchType::Remote
        } else {
            git2::BranchType::Local
        };

        Ok(Branch { name, locality })
    }
}

/// A newtype wrapper over `String` to separate out
/// the fact that a caller wants to fetch a tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagName(String);

impl TryFrom<&[u8]> for TagName {
    type Error = str::Utf8Error;

    fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(name).map(|name| Self(String::from(name)))
    }
}

impl TagName {
    /// Create a new `TagName`.
    pub fn new(name: &str) -> Self {
        TagName(name.into())
    }

    /// Access the string value of the `TagName`.
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// The static information of a `git2::Tag`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag {
    /// The Object ID for the `Tag`, i.e the SHA1 digest.
    pub id: git2::Oid,
    /// The name that references this `Tag`.
    pub name: TagName,
    /// The named author of this `Tag`, if the `Tag` was annotated.
    pub tagger: Option<Author>,
    /// The message with this `Tag`, if the `Tag` was annotated.
    pub message: Option<String>,
}

impl<'repo> TryFrom<git2::Tag<'repo>> for Tag {
    type Error = str::Utf8Error;

    fn try_from(tag: git2::Tag) -> Result<Self, Self::Error> {
        let id = tag.id();

        let name = TagName::try_from(tag.name_bytes())?;

        let tagger = tag.tagger().map(Author::try_from).transpose()?;

        let message = tag
            .message_bytes()
            .map(str::from_utf8)
            .transpose()?
            .map(|message| message.into());

        Ok(Tag {
            id,
            name,
            tagger,
            message,
        })
    }
}


pub enum RevObject {
    Branch(Branch),
    Tag(Tag),
    Commit(Commit),
}

impl RevObject {
    pub fn from_revparse(repo: &git2::Repository, spec: &str) -> Result<Self, Error> {
        let (object, optional_ref) = repo.revparse_ext(spec)?;

        match optional_ref {
            Some(reference) => match Branch::from_reference(&reference) {
                Some(branch_result) => Ok(RevObject::Branch(branch_result?)),
                None => Err(Error::RevParseFailure),
            },
            None => {
                let tag = object.into_tag().map(Tag::try_from);
                match tag {
                    Ok(tag) => Ok(RevObject::Tag(tag?)),
                    Err(object) => {
                        let commit = object.into_commit().map(Commit::try_from);
                        match commit {
                            Ok(commit) => Ok(RevObject::Commit(commit?)),
                            Err(_object) => Err(Error::RevParseFailure),
                        }
                    }
                }
            }
        }
    }
}