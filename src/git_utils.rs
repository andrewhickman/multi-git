use std::fmt;

use bstr::{BStr, BString, ByteSlice};
use git2::{Branch, ObjectType, Repository, Status, StatusOptions};

const HEAD_FILE: &str = "HEAD";
const REFS_HEADS_FILE: &str = "refs/heads/";

pub struct RepoStatus<'repo> {
    pub head: HeadStatus<'repo>,
    pub upstream: UpstreamStatus,
    pub working_tree: WorkingTreeStatus,
}

pub struct HeadStatus<'repo> {
    pub name: BString,
    pub kind: HeadStatusKind<'repo>,
}

pub enum HeadStatusKind<'repo> {
    Unborn,
    Detached,
    Branch { branch: Branch<'repo> },
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

fn get_head_status<'repo>(repo: &'repo Repository) -> Result<HeadStatus, git2::Error> {
    let head = repo.find_reference("HEAD")?;
    match head.symbolic_target_bytes() {
        // HEAD points to a branch
        Some(name) if name.starts_with(REFS_HEADS_FILE.as_bytes()) => {
            let name = name[REFS_HEADS_FILE.len()..].as_bstr().to_owned();
            match head.resolve() {
                Ok(branch) => Ok(HeadStatus {
                    name,
                    kind: HeadStatusKind::Branch {
                        branch: Branch::wrap(branch),
                    },
                }),
                Err(err)
                    if err.class() == git2::ErrorClass::Reference
                        && err.code() == git2::ErrorCode::NotFound =>
                {
                    Ok(HeadStatus {
                        name,
                        kind: HeadStatusKind::Unborn,
                    })
                }
                Err(err) => Err(err),
            }
        }
        // HEAD points to an oid (is detached)
        _ => {
            let object = head.peel(ObjectType::Any)?;
            let description = object.describe(
                &git2::DescribeOptions::new()
                    .describe_tags()
                    .show_commit_oid_as_fallback(true),
            )?;
            let name = description.format(None)?.into();
            Ok(HeadStatus {
                name,
                kind: HeadStatusKind::Detached,
            })
        }
    }
}

fn get_upstream_status(
    repo: &Repository,
    head: &HeadStatus,
) -> Result<UpstreamStatus, git2::Error> {
    let local_branch = match &head.kind {
        HeadStatusKind::Branch { branch } => branch,
        _ => return Ok(UpstreamStatus::None),
    };
    let local_oid = local_branch.get().peel(ObjectType::Any)?.id();

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

impl<'repo> fmt::Display for HeadStatus<'repo> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            HeadStatusKind::Unborn | HeadStatusKind::Branch { .. } => write!(f, "{}", self.name),
            HeadStatusKind::Detached => write!(f, "({})", self.name),
        }
    }
}