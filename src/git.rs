use std::path::Path;
use std::{fmt, str};

use bstr::{BString, ByteSlice};

use crate::config::Settings;

const HEAD_FILE: &str = "HEAD";
const REFS_HEADS_NAMESPACE: &str = "refs/heads/";

pub struct Repository {
    repo: git2::Repository,
}

pub struct RepositoryStatus {
    pub head: HeadStatus,
    pub upstream: UpstreamStatus,
    pub working_tree: WorkingTreeStatus,
    pub default_branch: Option<String>,
}

pub struct HeadStatus {
    pub name: BString,
    pub kind: HeadStatusKind,
}

pub enum HeadStatusKind {
    Unborn,
    Detached,
    Branch,
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
    UpToDate(String),
    CreatedUnborn(String),
    FastForwarded(String),
}

impl Repository {
    pub fn open(path: &Path) -> crate::Result<Self> {
        let repo = git2::Repository::open(path)?;
        log::debug!("opened repo at `{}`", path.display());
        Ok(Repository { repo })
    }

    pub fn clone<F>(
        path: &Path,
        repo: &str,
        settings: &Settings,
        mut progress_callback: F,
    ) -> crate::Result<Self>
    where
        F: FnMut(git2::Progress),
    {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.transfer_progress(|progress| {
            progress_callback(progress);
            true
        });

        let mut credentials_state = CredentialsState::default();
        callbacks.credentials(|url, username_from_url, allowed_types| {
            credentials_state.get(
                settings,
                &git2::Config::open_default()?,
                url,
                username_from_url,
                allowed_types,
            )
        });

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let repo = git2::build::RepoBuilder::new()
            .fetch_options(fetch_options)
            .clone(repo, path)?;

        log::debug!("cloned repo at `{}`", path.display());
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

    pub fn status(
        &self,
        settings: &Settings,
    ) -> crate::Result<(RepositoryStatus, Option<git2::Remote>)> {
        let head = self.head_status()?;
        let upstream = self.upstream_status(&head)?;
        let working_tree = self.working_tree_status()?;

        let (default_branch, remote) = self.try_default_branch(settings);

        Ok((
            RepositoryStatus {
                head,
                upstream,
                working_tree,
                default_branch,
            },
            remote,
        ))
    }

    fn head_status(&self) -> Result<HeadStatus, git2::Error> {
        let head = self.repo.find_reference(HEAD_FILE)?;
        match head.symbolic_target_bytes() {
            // HEAD points to a branch
            Some(name) if name.starts_with(REFS_HEADS_NAMESPACE.as_bytes()) => {
                let name = name[REFS_HEADS_NAMESPACE.len()..].as_bstr().to_owned();
                match head.resolve() {
                    Ok(_) => Ok(HeadStatus {
                        name,
                        kind: HeadStatusKind::Branch,
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

    fn upstream_status(&self, head_status: &HeadStatus) -> Result<UpstreamStatus, git2::Error> {
        let local_branch = if head_status.is_branch() {
            self.head_branch()?
        } else {
            return Ok(UpstreamStatus::None);
        };
        let local_oid = local_branch.get().peel_to_commit()?.id();

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
        let upstream_oid = upstream_branch.get().peel_to_commit()?.id();

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
        status: &RepositoryStatus,
        remote: Option<git2::Remote>,
        mut progress_callback: F,
    ) -> crate::Result<PullOutcome>
    where
        F: FnMut(git2::Progress),
    {
        let mut remote = match remote {
            Some(remote) => remote,
            None => self.default_remote(settings)?,
        };

        let repo_config = self.repo.config()?;

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.transfer_progress(|progress| {
            progress_callback(progress);
            true
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

        remote.fetch::<String>(
            &[],
            Some(
                &mut git2::FetchOptions::new()
                    .remote_callbacks(callbacks)
                    .download_tags(git2::AutotagOption::All)
                    .update_fetchhead(true)
                    .prune(prune),
            ),
            Some("multi-git: fetching"),
        )?;

        if !status.upstream.exists() {
            return Err(crate::Error::from_message("no upstream branch"));
        }

        if status.working_tree.is_dirty() {
            return Err(crate::Error::from_message(
                "working tree has uncommitted changes",
            ));
        }

        let default_branch = match &status.default_branch {
            Some(name) => name.clone(),
            None => self.default_branch_for_remote(&remote)?,
        };
        if !status.head.on_branch(&default_branch) {
            return Err(crate::Error::from_message("not on default branch"));
        }

        let upstream_oid = self
            .head_branch()?
            .upstream()?
            .into_reference()
            .target()
            .expect("branch is not direct reference");
        let fetch_head = self.repo.annotated_commit_from_fetchhead(
            &default_branch,
            remote.url().expect("remote url is invalid utf-8"),
            &upstream_oid,
        )?;

        let (merge_analysis, _) = self.repo.merge_analysis(&[&fetch_head])?;

        if merge_analysis.is_up_to_date() {
            Ok(PullOutcome::UpToDate(default_branch))
        } else if merge_analysis.is_unborn() {
            self.create_unborn(status, fetch_head)?;
            Ok(PullOutcome::CreatedUnborn(default_branch))
        } else if merge_analysis.is_fast_forward() {
            self.fast_forward(fetch_head)?;
            Ok(PullOutcome::FastForwarded(default_branch))
        } else {
            Err(crate::Error::from_message("cannot fast-forward"))
        }
    }

    fn create_unborn(
        &self,
        status: &RepositoryStatus,
        fetch_commit: git2::AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        debug_assert!(status.head.is_unborn());
        let branch_name = format!("{}{}", REFS_HEADS_NAMESPACE, status.head.name);
        let log_message = format!(
            "multi-git: creating unborn branch {} at {}",
            branch_name,
            fetch_commit.id()
        );
        self.repo
            .reference(&branch_name, fetch_commit.id(), false, &log_message)?;
        self.checkout(&branch_name)?;
        Ok(())
    }

    fn fast_forward(&self, fetch_commit: git2::AnnotatedCommit) -> Result<(), git2::Error> {
        let mut branch = self.head_branch()?;

        let log_message = format!(
            "multi-git: fast-forwarding branch {} to {}",
            branch.name_bytes()?.as_bstr(),
            fetch_commit.id(),
        );

        branch
            .get_mut()
            .set_target(fetch_commit.id(), &log_message)?;
        debug_assert!(branch.is_head());
        self.repo
            .checkout_head(Some(&mut git2::build::CheckoutBuilder::new().force()))?;
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
        let ref_name = format!("{}{}", REFS_HEADS_NAMESPACE, name);
        self.checkout(&ref_name)?;
        Ok(())
    }

    fn checkout(&self, ref_name: &str) -> Result<(), git2::Error> {
        self.repo.set_head(&ref_name)?;
        self.repo
            .checkout_head(Some(&mut git2::build::CheckoutBuilder::new().force()))?;
        Ok(())
    }

    fn head_branch(&self) -> Result<git2::Branch<'_>, git2::Error> {
        let head = self.repo.head()?;
        debug_assert!(head.is_branch());
        Ok(git2::Branch::wrap(head))
    }

    fn default_remote(&self, settings: &Settings) -> Result<git2::Remote, crate::Error> {
        let remote_list = self.repo.remotes()?;
        let remote_name = match &settings.default_remote {
            Some(default_branch) => default_branch,
            None => match remote_list.len() {
                0 => return Err(crate::Error::from_message("no remotes")),
                1 => match remote_list.get(0) {
                    Some(name) => name,
                    None => {
                        return Err(crate::Error::from_message(
                            "default remote name is invalid utf-8",
                        ))
                    }
                },
                _ => return Err(crate::Error::from_message("no default remote")),
            },
        };

        Ok(self.repo.find_remote(remote_name)?)
    }

    fn default_branch_for_remote(&self, remote: &git2::Remote) -> Result<String, crate::Error> {
        let name = remote.default_branch()?;
        match str::from_utf8(name.as_ref()) {
            Ok(name) => Ok(name
                .strip_prefix(REFS_HEADS_NAMESPACE)
                .unwrap_or(name)
                .to_owned()),
            Err(_) => Err(crate::Error::from_message(
                "default branch name is invalid utf-8",
            )),
        }
    }

    fn try_default_branch(&self, settings: &Settings) -> (Option<String>, Option<git2::Remote>) {
        if let Some(name) = &settings.default_branch {
            return (Some(name.to_owned()), None);
        }

        self.default_remote(settings)
            .and_then(|mut remote| {
                let mut callbacks = git2::RemoteCallbacks::new();
                let mut credentials_state = CredentialsState::default();
                callbacks.credentials(|url, username_from_url, allowed_types| {
                    credentials_state.get(
                        settings,
                        &git2::Config::open_default()?,
                        url,
                        username_from_url,
                        allowed_types,
                    )
                });

                let _ = remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

                let default_branch = self.default_branch_for_remote(&remote)?;
                Ok((Some(default_branch), Some(remote)))
            })
            .unwrap_or((None, None))
    }
}

impl RepositoryStatus {
    pub fn on_default_branch(&self) -> bool {
        match &self.default_branch {
            Some(name) => self.head.on_branch(&name),
            None => false,
        }
    }
}

impl HeadStatus {
    fn is_branch(&self) -> bool {
        match self.kind {
            HeadStatusKind::Branch => true,
            _ => false,
        }
    }

    fn is_unborn(&self) -> bool {
        match self.kind {
            HeadStatusKind::Unborn => true,
            _ => false,
        }
    }

    pub fn on_branch(&self, name: impl AsRef<[u8]>) -> bool {
        match &self.kind {
            HeadStatusKind::Branch | HeadStatusKind::Unborn => &self.name == name.as_ref(),
            HeadStatusKind::Detached => false,
        }
    }
}

impl fmt::Display for HeadStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            HeadStatusKind::Unborn | HeadStatusKind::Branch => write!(f, "{}", self.name),
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
