use std::fmt;

use bstr::{BStr, BString, ByteSlice};
use git2::{Branch, ObjectType, Oid, Reference, Repository, Status, StatusOptions};

pub struct RepoStatus<'repo> {
    pub head: HeadStatus<'repo>,
    pub upstream: UpstreamStatus,
    pub working_tree: WorkingTreeStatus,
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
    Gone,
}

pub struct WorkingTreeStatus {
    pub working_changed: bool,
    pub index_changed: bool,
}

pub fn get_status<'repo>(repo: &'repo mut Repository) -> Result<RepoStatus<'repo>, git2::Error> {
    let head = get_head_status(repo)?;
    let upstream = get_upstream_status(repo, &head)?;
    let working_tree = get_working_tree_status(repo)?;

    Ok(RepoStatus {
        head,
        upstream,
        working_tree,
    })
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
        Err(err) => {
            return match (err.code(), err.class()) {
                // No upstream is set in the config
                (git2::ErrorCode::NotFound, git2::ErrorClass::Config) => Ok(UpstreamStatus::None),
                // The upstream is set in the config but no longer exists.
                (git2::ErrorCode::NotFound, git2::ErrorClass::Reference) => {
                    Ok(UpstreamStatus::Gone)
                }
                _ => Err(err),
            };
        }
    };
    let upstream_oid = upstream_branch.get().peel(ObjectType::Any)?.id();

    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;

    Ok(UpstreamStatus::Upstream { ahead, behind })
}

fn get_working_tree_status(repo: &Repository) -> Result<WorkingTreeStatus, git2::Error> {
    let statuses = repo.statuses(Some(&mut StatusOptions::new().exclude_submodules(true)))?;

    let mut result = WorkingTreeStatus {
        working_changed: false,
        index_changed: false,
    };

    let working_changed_mask = Status::WT_NEW
        | Status::WT_MODIFIED
        | Status::WT_DELETED
        | Status::WT_RENAMED
        | Status::WT_TYPECHANGE;
    let index_changed_mask = Status::INDEX_NEW
        | Status::INDEX_MODIFIED
        | Status::INDEX_DELETED
        | Status::INDEX_RENAMED
        | Status::INDEX_TYPECHANGE;

    for entry in statuses.iter() {
        let status = entry.status();

        result.working_changed |= status.intersects(working_changed_mask);
        result.index_changed |= status.intersects(index_changed_mask);
    }

    Ok(result)
}

impl<'repo> HeadStatus<'repo> {
    pub fn name(&self) -> &BStr {
        match self {
            HeadStatus::Unborn => "master".as_bytes().as_bstr(),
            HeadStatus::Detached { name, .. } => name.as_ref(),
            HeadStatus::Branch { name, .. } => name.as_ref(),
        }
    }
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
