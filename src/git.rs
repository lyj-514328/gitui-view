use anyhow::{Context, Result};
use git2::{DiffOptions, Repository, Sort};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct StatusEntry {
    pub path: String,
    pub status: StatusType,
    pub staged: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    TypeChange,
}

#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub author: String,
    pub time: i64,
    pub message: String,
    pub summary: String,
}

#[derive(Clone, Debug)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
    pub commit_id: String,
    pub time: i64,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub content: String,
    pub line_type: DiffLineType,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Add,
    Delete,
    Header,
}

#[derive(Clone, Debug)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug)]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub status: StatusType,
    pub hunks: Vec<Hunk>,
    pub binary: bool,
}

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::open(path)
            .with_context(|| format!("Failed to open git repository at {:?}", path))?;
        Ok(Self { repo })
    }

    pub fn open_from_cwd() -> Result<Self> {
        let repo = Repository::open_from_env()
            .context("Failed to open git repository from current directory")?;
        Ok(Self { repo })
    }

    pub fn workdir(&self) -> Option<PathBuf> {
        self.repo.workdir().map(|p| p.to_path_buf())
    }

    pub fn get_status(&self) -> Result<(Vec<StatusEntry>, Vec<StatusEntry>)> {
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();

        let mut status_opts = git2::StatusOptions::new();
        status_opts
            .include_untracked(true)
            .recurse_untracked_dirs(true);

        let statuses = self
            .repo
            .statuses(Some(&mut status_opts))
            .context("Failed to get repository status")?;

        for entry in statuses.iter() {
            let path = entry
                .path()
                .map(|p| p.to_string())
                .unwrap_or_default();

            let flags = entry.status();

            if flags.is_empty() {
                continue;
            }

            if flags.intersects(
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE,
            ) {
                let st = if flags.contains(git2::Status::INDEX_NEW) {
                    StatusType::Added
                } else if flags.contains(git2::Status::INDEX_DELETED) {
                    StatusType::Deleted
                } else if flags.contains(git2::Status::INDEX_RENAMED) {
                    StatusType::Renamed
                } else if flags.contains(git2::Status::INDEX_TYPECHANGE) {
                    StatusType::TypeChange
                } else {
                    StatusType::Modified
                };
                staged.push(StatusEntry {
                    path: path.clone(),
                    status: st,
                    staged: true,
                });
            }

            if flags.intersects(
                git2::Status::WT_NEW
                    | git2::Status::WT_MODIFIED
                    | git2::Status::WT_DELETED
                    | git2::Status::WT_RENAMED
                    | git2::Status::WT_TYPECHANGE,
            ) {
                let st = if flags.contains(git2::Status::WT_NEW) {
                    StatusType::Untracked
                } else if flags.contains(git2::Status::WT_DELETED) {
                    StatusType::Deleted
                } else if flags.contains(git2::Status::WT_RENAMED) {
                    StatusType::Renamed
                } else if flags.contains(git2::Status::WT_TYPECHANGE) {
                    StatusType::TypeChange
                } else {
                    StatusType::Modified
                };
                unstaged.push(StatusEntry {
                    path,
                    status: st,
                    staged: false,
                });
            }
        }

        Ok((staged, unstaged))
    }

    pub fn get_commits(&self, count: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk().context("Failed to create revwalk")?;
        revwalk.set_sorting(Sort::TIME)?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        for (_, oid) in revwalk.enumerate().take(count) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let time = commit.time().seconds();
            let author = commit.author().name().unwrap_or("unknown").to_string();
            let message = commit.message().unwrap_or("").to_string();
            let summary = commit.summary().unwrap_or("").to_string();

            commits.push(CommitInfo {
                id: oid.to_string(),
                short_id: oid.to_string()[..7].to_string(),
                author,
                time,
                message,
                summary,
            });
        }

        Ok(commits)
    }

    pub fn get_commit_diff(&self, commit_id: &str) -> Result<Vec<FileDiff>> {
        let oid = git2::Oid::from_str(commit_id)
            .map_err(|_| anyhow::anyhow!("Invalid commit id: {}", commit_id))?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            commit.parent(0)?.tree().ok()
        } else {
            None
        };

        let mut diff_opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        Self::diff_to_file_diffs(&diff)
    }

    pub fn get_workdir_diff(&self, path: &str, staged: bool) -> Result<FileDiff> {
        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(path);

        let diff = if staged {
            let head_tree = self.repo.head().ok().and_then(|h| {
                h.peel_to_tree().ok()
            });
            let index = self.repo.index()?;
            self.repo
                .diff_tree_to_index(head_tree.as_ref(), Some(&index), Some(&mut diff_opts))?
        } else {
            diff_opts.include_untracked(true);
            diff_opts.recurse_untracked_dirs(true);
            let index = self.repo.index()?;
            self.repo
                .diff_index_to_workdir(Some(&index), Some(&mut diff_opts))?
        };

        let mut file_diffs = Self::diff_to_file_diffs(&diff)?;
        Ok(file_diffs.remove(0))
    }

    pub fn get_diff_for_file(&self, path: &str) -> Result<Vec<FileDiff>> {
        let mut file_diffs = Vec::new();

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(path);

        let head_tree = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        let index = self.repo.index()?;

        let diff_staged = self.repo.diff_tree_to_index(
            head_tree.as_ref(),
            Some(&index),
            Some(&mut diff_opts),
        )?;
        file_diffs.extend(Self::diff_to_file_diffs(&diff_staged)?);

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(path);
        diff_opts.include_untracked(true);
        diff_opts.recurse_untracked_dirs(true);

        let diff_unstaged = self
            .repo
            .diff_index_to_workdir(Some(&index), Some(&mut diff_opts))?;
        file_diffs.extend(Self::diff_to_file_diffs(&diff_unstaged)?);

        Ok(file_diffs)
    }

    pub fn get_stashes(&mut self) -> Result<Vec<StashInfo>> {
        let mut stashes = Vec::new();
        self.repo.stash_foreach(|index, message, oid| {
            stashes.push(StashInfo {
                index,
                message: message.to_string(),
                commit_id: oid.to_string(),
                time: 0,
            });
            true
        })?;

        for stash in &mut stashes {
            if let Ok(commit) = self.repo.find_commit(
                git2::Oid::from_str(&stash.commit_id).unwrap(),
            ) {
                stash.time = commit.time().seconds();
            }
        }

        Ok(stashes)
    }

    pub fn get_stash_diff(&mut self, index: usize) -> Result<Vec<FileDiff>> {
        let mut stashes = Vec::new();
        self.repo.stash_foreach(|i, _msg, oid| {
            stashes.push((i, *oid));
            true
        })?;

        let stash_oid = stashes
            .into_iter()
            .find(|(i, _)| *i == index)
            .map(|(_, oid)| oid)
            .ok_or_else(|| anyhow::anyhow!("Stash index {} not found", index))?;

        let commit = self.repo.find_commit(stash_oid)?;
        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            commit.parent(0)?.tree().ok()
        } else {
            None
        };

        let mut diff_opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        Self::diff_to_file_diffs(&diff)
    }

    fn diff_to_file_diffs(diff: &git2::Diff) -> Result<Vec<FileDiff>> {
        let file_diffs = RefCell::new(Vec::new());

        diff.foreach(
            &mut |delta, _| {
                let old_path = delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let new_path = delta
                    .new_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let status = match delta.status() {
                    git2::Delta::Added => StatusType::Added,
                    git2::Delta::Deleted => StatusType::Deleted,
                    git2::Delta::Modified => StatusType::Modified,
                    git2::Delta::Renamed => StatusType::Renamed,
                    git2::Delta::Copied => StatusType::Copied,
                    git2::Delta::Untracked => StatusType::Untracked,
                    git2::Delta::Typechange => StatusType::TypeChange,
                    _ => StatusType::Modified,
                };

                let binary = delta.flags().contains(git2::DiffFlags::BINARY);

                file_diffs.borrow_mut().push(FileDiff {
                    old_path,
                    new_path,
                    status,
                    hunks: Vec::new(),
                    binary,
                });

                true
            },
            None,
            Some(&mut |_delta, hunk| {
                let header = String::from_utf8_lossy(hunk.header()).to_string();
                file_diffs.borrow_mut().last_mut().unwrap().hunks.push(Hunk {
                    header,
                    lines: Vec::new(),
                });
                true
            }),
            Some(&mut |_delta, _hunk, line| {
                let content = String::from_utf8_lossy(line.content())
                    .trim_end_matches('\n')
                    .to_string();

                let line_type = match line.origin() {
                    '+' => DiffLineType::Add,
                    '-' => DiffLineType::Delete,
                    ' ' => DiffLineType::Context,
                    _ => DiffLineType::Header,
                };

                if let Some(hunk) = file_diffs.borrow_mut().last_mut().unwrap().hunks.last_mut() {
                    hunk.lines.push(DiffLine {
                        content,
                        line_type,
                        old_lineno: line.old_lineno(),
                        new_lineno: line.new_lineno(),
                    });
                }

                true
            }),
        )?;

        Ok(file_diffs.into_inner())
    }
}
