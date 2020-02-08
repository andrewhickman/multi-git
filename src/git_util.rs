use std::fmt;

use bstr::{BString, ByteSlice};
use git2::{Branch, ObjectType, Oid, Reference, Repository};

pub struct Status<'repo> {
    pub head: HeadStatus<'repo>,
    pub upstream: UpstreamStatus,
}

pub enum HeadStatus<'repo> {
    Detached {
        name: BString,
        head: Reference<'repo>,
    },
    Branch {
        name: BString,
        branch: Branch<'repo>,
        oid: Oid,
    },
    Unborn,
}

pub enum UpstreamStatus {
    None,
    Upstream { ahead: usize, behind: usize },
}

pub fn get_status<'repo>(repo: &'repo mut Repository) -> Result<Status<'repo>, git2::Error> {
    let head = get_head_status(repo)?;
    let upstream = get_upstream_status(repo, &head)?;

    Ok(Status { head, upstream })
}

fn get_head_status<'repo>(repo: &'repo Repository) -> Result<HeadStatus<'repo>, git2::Error> {
    let detached = repo.head_detached()?;
    match repo.head() {
        Ok(head) => {
            let object = head.peel(ObjectType::Any)?;
            if detached {
                let description = object.describe(
                    &git2::DescribeOptions::new()
                        .describe_all()
                        .show_commit_oid_as_fallback(true),
                )?;

                Ok(HeadStatus::Detached {
                    name: description.format(None)?.into(),
                    head,
                })
            } else {
                Ok(HeadStatus::Branch {
                    name: head.shorthand_bytes().as_bstr().to_owned(),
                    branch: Branch::wrap(head),
                    oid: object.id(),
                })
            }
        }
        Err(err) if err.code() == git2::ErrorCode::UnbornBranch => Ok(HeadStatus::Unborn),
        Err(err) => return Err(err),
    }
}

fn get_upstream_status(
    repo: &Repository,
    head: &HeadStatus,
) -> Result<UpstreamStatus, git2::Error> {
    let (local_branch, local_oid) = match head {
        HeadStatus::Branch { branch, oid, .. } => (branch, *oid),
        _ => return Ok(UpstreamStatus::None),
    };

    let upstream_branch = match local_branch.upstream() {
        Ok(branch) => branch,
        Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(UpstreamStatus::None),
        Err(err) => return Err(err),
    };
    let upstream_oid = upstream_branch.get().peel(ObjectType::Any)?.id();

    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;

    Ok(UpstreamStatus::Upstream { ahead, behind })
}

impl<'repo> fmt::Display for HeadStatus<'repo> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HeadStatus::Unborn => write!(f, "master"),
            HeadStatus::Detached { name, .. } => write!(f, "({})", name),
            HeadStatus::Branch { name, .. } => write!(f, "{}", name),
        }
    }
}
