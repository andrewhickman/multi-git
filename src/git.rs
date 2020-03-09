use std::fmt;
use std::path::Path;

use bstr::{BString, ByteSlice};

use crate::config::Settings;

const HEAD_FILE: &str = "HEAD";
const REFS_HEADS_FILE: &str = "refs/heads/";

pub struct Repository {
    repo: git2::Repository,
}

pub struct RepositoryStatus<'repo> {
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
    Branch { branch: git2::Branch<'repo> },
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

pub enum PullOutcome {
    UpToDate,
    CreatedUnborn,
    FastForwarded,
}

impl Repository {
    pub fn open(path: &Path) -> crate::Result<Self> {
        let repo = git2::Repository::open(path)?;
        log::debug!("opened repo at `{}`", path.display());
        Ok(Repository { repo })
    }

    pub fn try_open(path: &Path) -> crate::Result<Option<Self>> {
        match git2::Repository::open(path) {
            Ok(repo) => {
                log::debug!("opened repo at `{}`", path.display());
                Ok(Some(Repository { repo }))
            }
            Err(err)
                if err.class() == git2::ErrorClass::Repository
                    && err.code() == git2::ErrorCode::NotFound =>
            {
                Ok(None)
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn status(&self) -> crate::Result<RepositoryStatus<'_>> {
        let head = self.head_status()?;
        let upstream = self.upstream_status(&head)?;
        let working_tree = self.working_tree_status()?;

        Ok(RepositoryStatus {
            head,
            upstream,
            working_tree,
        })
    }

    fn head_status(&self) -> Result<HeadStatus, git2::Error> {
        let head = self.repo.find_reference(HEAD_FILE)?;
        match head.symbolic_target_bytes() {
            // HEAD points to a branch
            Some(name) if name.starts_with(REFS_HEADS_FILE.as_bytes()) => {
                let name = name[REFS_HEADS_FILE.len()..].as_bstr().to_owned();
                match head.resolve() {
                    Ok(branch) => Ok(HeadStatus {
                        name,
                        kind: HeadStatusKind::Branch {
                            branch: git2::Branch::wrap(branch),
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
                let object = head.peel(git2::ObjectType::Any)?;
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

    fn upstream_status(&self, head: &HeadStatus) -> Result<UpstreamStatus, git2::Error> {
        let local_branch = match &head.kind {
            HeadStatusKind::Branch { branch } => branch,
            _ => return Ok(UpstreamStatus::None),
        };
        let local_oid = local_branch.get().peel(git2::ObjectType::Any)?.id();

        let upstream_branch = match local_branch.upstream() {
            Ok(branch) => branch,
            Err(err) => {
                return match (err.code(), err.class()) {
                    // No upstream is set in the config
                    (git2::ErrorCode::NotFound, git2::ErrorClass::Config) => {
                        Ok(UpstreamStatus::None)
                    }
                    // The upstream is set in the config but no longer exists.
                    (git2::ErrorCode::NotFound, git2::ErrorClass::Reference) => {
                        Ok(UpstreamStatus::Gone)
                    }
                    _ => Err(err),
                };
            }
        };
        let upstream_oid = upstream_branch.get().peel(git2::ObjectType::Any)?.id();

        let (ahead, behind) = self.repo.graph_ahead_behind(local_oid, upstream_oid)?;

        Ok(UpstreamStatus::Upstream { ahead, behind })
    }

    fn working_tree_status(&self) -> Result<WorkingTreeStatus, git2::Error> {
        let statuses = self.repo.statuses(Some(
            &mut git2::StatusOptions::new()
                .exclude_submodules(true)
                .include_ignored(false),
        ))?;

        let mut result = WorkingTreeStatus {
            working_changed: false,
            index_changed: false,
        };

        let working_changed_mask = git2::Status::WT_NEW
            | git2::Status::WT_MODIFIED
            | git2::Status::WT_DELETED
            | git2::Status::WT_RENAMED
            | git2::Status::WT_TYPECHANGE;
        let index_changed_mask = git2::Status::INDEX_NEW
            | git2::Status::INDEX_MODIFIED
            | git2::Status::INDEX_DELETED
            | git2::Status::INDEX_RENAMED
            | git2::Status::INDEX_TYPECHANGE
            | git2::Status::CONFLICTED;

        for entry in statuses.iter() {
            let status = entry.status();

            result.working_changed |= status.intersects(working_changed_mask);
            result.index_changed |= status.intersects(index_changed_mask);
        }

        Ok(result)
    }

    pub fn pull<F>(
        &self,
        settings: &Settings,
        status: &mut RepositoryStatus,
        mut progress_callback: F,
    ) -> crate::Result<PullOutcome>
    where
        F: FnMut(git2::Progress) -> crate::Result<bool>,
    {
        let branch_name = match &settings.default_branch {
            Some(default_branch) => default_branch,
            None => return Err(crate::Error::from_message("no default branch")),
        };

        let remote_name = match &settings.default_remote {
            Some(default_branch) => default_branch,
            None => return Err(crate::Error::from_message("no default remote")),
        };

        let mut remote = self.repo.find_remote(remote_name)?;

        let repo_config = self.repo.config()?;

        let mut result = Ok(());
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.transfer_progress(|progress| match progress_callback(progress) {
            Ok(result) => result,
            Err(err) => {
                result = Err(err);
                false
            }
        });

        let mut credentials_state = CredentialsState::default();
        callbacks.credentials(|url, username_from_url, allowed_types| {
            credentials_state.get(
                settings,
                &repo_config,
                url,
                username_from_url,
                allowed_types,
            )
        });

        let prune = match settings.prune {
            None => git2::FetchPrune::Unspecified,
            Some(false) => git2::FetchPrune::Off,
            Some(true) => git2::FetchPrune::On,
        };

        remote.fetch(
            &[branch_name],
            Some(
                &mut git2::FetchOptions::new()
                    .remote_callbacks(callbacks)
                    .download_tags(git2::AutotagOption::All)
                    .update_fetchhead(true)
                    .prune(prune),
            ),
            Some("multi-git: fetching"),
        )?;
        result?;

        if !status.upstream.exists() {
            return Err(crate::Error::from_message("no upstream branch"));
        }

        if status.working_tree.is_dirty() {
            return Err(crate::Error::from_message(
                "working tree has uncommitted changes",
            ));
        }

        if !status.head.on_default_branch(settings) {
            return Err(crate::Error::from_message("not on default branch"));
        }

        let upstream_oid = status
            .head
            .unwrap_branch()
            .upstream()?
            .into_reference()
            .target()
            .expect("branch is not direct reference");
        let fetch_head = self.repo.annotated_commit_from_fetchhead(
            branch_name,
            remote.url().expect("remote url is invalid utf-8"),
            &upstream_oid,
        )?;

        let (merge_analysis, _) = self.repo.merge_analysis(&[&fetch_head])?;

        if merge_analysis.is_up_to_date() {
            Ok(PullOutcome::UpToDate)
        } else if merge_analysis.is_unborn() {
            self.create_unborn(status, fetch_head)?;
            Ok(PullOutcome::CreatedUnborn)
        } else if merge_analysis.is_fast_forward() {
            self.fast_forward(status, fetch_head)?;
            Ok(PullOutcome::FastForwarded)
        } else {
            Err(crate::Error::from_message("cannot fast-forward"))
        }
    }

    fn create_unborn(
        &self,
        status: &RepositoryStatus<'_>,
        fetch_commit: git2::AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        debug_assert!(status.head.is_unborn());
        let branch_name = format!("{}{}", REFS_HEADS_FILE, status.head.name);
        let log_message = format!(
            "multi-git: creating unborn branch {} at {}",
            branch_name,
            fetch_commit.id()
        );
        self.repo
            .reference(&branch_name, fetch_commit.id(), false, &log_message)?;
        self.repo.set_head(&branch_name)?;
        self.repo.checkout_head(Some(
            &mut git2::build::CheckoutBuilder::new()
                .force()
                .remove_untracked(true),
        ))?;
        Ok(())
    }

    fn fast_forward(
        &self,
        status: &mut RepositoryStatus<'_>,
        fetch_commit: git2::AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        let branch = status.head.unwrap_branch();

        let log_message = format!(
            "multi-git: fast-forwarding branch {} to {}",
            branch.name_bytes()?.as_bstr(),
            fetch_commit.id(),
        );

        branch
            .get_mut()
            .set_target(fetch_commit.id(), &log_message)?;
        debug_assert!(branch.is_head());
        self.repo.checkout_head(Some(
            &mut git2::build::CheckoutBuilder::new()
                .force()
                .remove_untracked(true),
        ))?;
        Ok(())
    }

    pub fn create_branch(&self, settings: &Settings, name: &str) -> crate::Result<()> {
        let commit = match &settings.default_branch {
            Some(default_branch) => self
                .repo
                .find_branch(default_branch, git2::BranchType::Local)?
                .get()
                .peel_to_commit()?,
            None => self.repo.head()?.peel_to_commit()?,
        };

        let working_tree_status = self.working_tree_status()?;
        if working_tree_status.is_dirty() {
            return Err(crate::Error::from_message(
                "working tree has uncommitted changes",
            ));
        }

        self.repo.branch(name, &commit, false)?;
        let ref_name = format!("{}{}", REFS_HEADS_FILE, name);

        self.repo.set_head(&ref_name)?;
        self.repo.checkout_head(Some(
            &mut git2::build::CheckoutBuilder::new()
                .force()
                .remove_untracked(true),
        ))?;
        Ok(())
    }
}

impl<'repo> HeadStatus<'repo> {
    fn is_unborn(&self) -> bool {
        match self.kind {
            HeadStatusKind::Unborn => true,
            _ => false,
        }
    }

    fn unwrap_branch(&mut self) -> &mut git2::Branch<'repo> {
        match &mut self.kind {
            HeadStatusKind::Branch { branch } => branch,
            _ => panic!("expected HEAD to be on a branch"),
        }
    }

    pub fn on_default_branch(&self, settings: &Settings) -> bool {
        match &self.kind {
            HeadStatusKind::Branch { .. } | HeadStatusKind::Unborn => {
                match &settings.default_branch {
                    Some(branch) => branch.as_bytes() == self.name,
                    None => true,
                }
            }
            HeadStatusKind::Detached => false,
        }
    }
}

impl<'repo> fmt::Display for HeadStatus<'repo> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            HeadStatusKind::Unborn | HeadStatusKind::Branch { .. } => write!(f, "{}", self.name),
            HeadStatusKind::Detached => write!(f, "({})", self.name),
        }
    }
}

impl UpstreamStatus {
    pub fn exists(&self) -> bool {
        match self {
            UpstreamStatus::Upstream { .. } => true,
            UpstreamStatus::None | UpstreamStatus::Gone => false,
        }
    }
}

impl WorkingTreeStatus {
    pub fn is_dirty(&self) -> bool {
        self.index_changed || self.working_changed
    }
}

#[derive(Debug, Default)]
struct CredentialsState {
    tried_ssh_key_from_agent: bool,
    tried_ssh_key_from_config: bool,
    ssh_username_requested: bool,
    tried_cred_helper: bool,
}

impl CredentialsState {
    pub fn get(
        &mut self,
        settings: &Settings,
        repo_config: &git2::Config,
        url: &str,
        username_from_url: Option<&str>,
        allowed_types: git2::CredentialType,
    ) -> Result<git2::Cred, git2::Error> {
        if allowed_types.contains(git2::CredentialType::USERNAME) {
            debug_assert!(username_from_url.is_none());
            self.ssh_username_requested = true;
        }

        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            debug_assert!(!self.ssh_username_requested);
            let username = username_from_url.unwrap();

            if !self.tried_ssh_key_from_config {
                self.tried_ssh_key_from_config = true;
                if let Some(ssh) = &settings.ssh {
                    return git2::Cred::ssh_key(
                        username,
                        ssh.public_key_path.as_deref(),
                        &ssh.private_key_path,
                        ssh.passphrase.as_deref(),
                    );
                }
            }

            if !self.tried_ssh_key_from_agent {
                self.tried_ssh_key_from_agent = true;
                return git2::Cred::ssh_key_from_agent(username);
            }
        }

        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT)
            && !self.tried_cred_helper
        {
            self.tried_cred_helper = true;
            return git2::Cred::credential_helper(repo_config, url, username_from_url);
        }

        if allowed_types.contains(git2::CredentialType::DEFAULT) {
            return git2::Cred::default();
        }

        Err(git2::Error::from_str("no credentials found"))
    }
}
