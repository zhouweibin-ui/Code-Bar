use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::runtime_scope::session_worktree_root_dir;
use crate::util::{background_command, expand_path, normalize_expanded_path};

// ── 辅助函数 ──────────────────────────────────────────────────────
fn is_svn_working_copy(workdir: &Path) -> bool {
    let mut current = Some(workdir);
    while let Some(path) = current {
        if path.join(".svn").exists() {
            return true;
        }
        if path.join(".git").exists() {
            return false;
        }
        current = path.parent();
    }
    false
}

pub fn session_branch_prefix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let token = format!("{:06x}", millis % 0x1000000);
    format!("ci/{token}")
}

pub fn session_branch_name(prefix: &str, session_id: &str) -> String {
    format!("{prefix}/session-{session_id}")
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDeleteStatusCounts {
    staged_count: usize,
    unstaged_count: usize,
    untracked_count: usize,
    conflict_count: usize,
}

impl SessionDeleteStatusCounts {
    fn has_uncommitted_changes(&self) -> bool {
        self.staged_count > 0
            || self.unstaged_count > 0
            || self.untracked_count > 0
            || self.conflict_count > 0
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDeleteSafety {
    staged_count: usize,
    unstaged_count: usize,
    untracked_count: usize,
    conflict_count: usize,
    ahead_count: usize,
    has_uncommitted_changes: bool,
    has_unmerged_commits: bool,
}

fn parse_session_delete_status_counts(status_output: &str) -> SessionDeleteStatusCounts {
    let mut counts = SessionDeleteStatusCounts::default();

    for line in status_output.lines() {
        if line.len() < 3 {
            continue;
        }

        let chars = line.chars().collect::<Vec<_>>();
        let x = chars[0];
        let y = chars[1];
        let conflicted = matches!(
            (x, y),
            ('D', 'D')
                | ('A', 'U')
                | ('U', 'D')
                | ('U', 'A')
                | ('D', 'U')
                | ('A', 'A')
                | ('U', 'U')
        ) || x == 'U'
            || y == 'U';
        let untracked = x == '?' && y == '?';
        let staged = !conflicted && !untracked && x != ' ';
        let unstaged = !conflicted && !untracked && y != ' ';

        if conflicted {
            counts.conflict_count += 1;
            continue;
        }
        if untracked {
            counts.untracked_count += 1;
            continue;
        }
        if staged {
            counts.staged_count += 1;
        }
        if unstaged {
            counts.unstaged_count += 1;
        }
    }

    counts
}

fn parse_rev_count(output: &[u8]) -> usize {
    String::from_utf8_lossy(output)
        .trim()
        .parse::<usize>()
        .unwrap_or(0)
}

/// 从 worktree 目录的 HEAD 文件读取分支名
pub fn read_worktree_branch(worktree_path: &Path) -> Option<String> {
    // worktree 中的 HEAD 格式：ref: refs/heads/<branch>
    let content = fs::read_to_string(worktree_path.join("HEAD")).ok()?;
    let branch = content.trim().strip_prefix("ref: refs/heads/")?;
    Some(branch.to_string())
}

/// 强制移除 worktree（先尝试 git worktree remove，失败则手动删目录 + prune）
fn force_remove_worktree(workdir: &str, wt_path: &str) {
    let _ = background_command("git")
        .current_dir(workdir)
        .args(["worktree", "remove", "--force", wt_path])
        .output();

    let p = Path::new(wt_path);
    if p.exists() {
        let _ = fs::remove_dir_all(p);
        let _ = background_command("git")
            .current_dir(workdir)
            .args(["worktree", "prune"])
            .output();
    }
}

// ── Tauri Commands ────────────────────────────────────────────────

/// 创建 git worktree（基于当前 HEAD 创建新分支并 checkout 到指定路径）
#[tauri::command]
pub async fn git_worktree_create(
    workdir: String,
    branch: String,
    worktree_path: String,
) -> Result<String, String> {
    let expanded_workdir = expand_path(&workdir);
    let expanded_wt_path = expand_path(&worktree_path);

    tokio::task::spawn_blocking(move || {
        if let Some(parent) = Path::new(&expanded_wt_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }

        let out = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["worktree", "add", "-b", &branch, &expanded_wt_path, "HEAD"])
            .output()
            .map_err(|e| e.to_string())?;

        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        Ok(expanded_wt_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 删除 git worktree（可选同时删除对应分支）
#[tauri::command]
pub async fn git_worktree_remove(
    workdir: String,
    worktree_path: String,
    branch: String,
    delete_branch: bool,
) -> Result<(), String> {
    let expanded_workdir = expand_path(&workdir);
    let expanded_wt_path = expand_path(&worktree_path);

    tokio::task::spawn_blocking(move || {
        force_remove_worktree(&expanded_workdir, &expanded_wt_path);

        if delete_branch && !branch.is_empty() {
            let _ = background_command("git")
                .current_dir(&expanded_workdir)
                .args(["branch", "-D", &branch])
                .output();
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 列出所有 git worktree（返回结构化信息）
#[tauri::command]
pub async fn git_worktree_list(workdir: String) -> Result<Vec<serde_json::Value>, String> {
    let expanded = expand_path(&workdir);

    tokio::task::spawn_blocking(move || {
        let out = background_command("git")
            .current_dir(&expanded)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .map_err(|e| e.to_string())?;

        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut worktrees = vec![];
        let mut current: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

        for line in stdout.lines() {
            if line.is_empty() {
                if !current.is_empty() {
                    worktrees.push(serde_json::Value::Object(current.clone()));
                    current.clear();
                }
            } else if let Some(path) = line.strip_prefix("worktree ") {
                current.insert("path".into(), path.into());
            } else if let Some(hash) = line.strip_prefix("HEAD ") {
                current.insert("head".into(), hash.into());
            } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
                current.insert("branch".into(), branch.into());
            } else if line == "bare" {
                current.insert("bare".into(), true.into());
            } else if line == "detached" {
                current.insert("detached".into(), true.into());
            }
        }
        if !current.is_empty() {
            worktrees.push(serde_json::Value::Object(current));
        }

        Ok(worktrees)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 将 worktree 分支 merge 回目标分支，然后删除 worktree 和分支
#[tauri::command]
pub async fn git_worktree_merge(
    workdir: String,
    worktree_path: String,
    branch: String,
    target_branch: String,
) -> Result<(), String> {
    let expanded_workdir = expand_path(&workdir);
    let expanded_wt_path = expand_path(&worktree_path);

    tokio::task::spawn_blocking(move || {
        // 切换到目标分支
        let switch = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["checkout", &target_branch])
            .output()
            .map_err(|e| e.to_string())?;
        if !switch.status.success() {
            return Err(format!(
                "切换到 {} 失败: {}",
                target_branch,
                String::from_utf8_lossy(&switch.stderr).trim()
            ));
        }

        // merge --no-ff
        let merge = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["merge", "--no-ff", &branch])
            .output()
            .map_err(|e| e.to_string())?;
        if !merge.status.success() {
            return Err(format!(
                "merge 失败: {}",
                String::from_utf8_lossy(&merge.stderr).trim()
            ));
        }

        // 删除 worktree 和分支
        force_remove_worktree(&expanded_workdir, &expanded_wt_path);
        if !branch.is_empty() {
            let _ = background_command("git")
                .current_dir(&expanded_workdir)
                .args(["branch", "-D", &branch])
                .output();
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 为 session 自动创建独立 git worktree
/// 返回 { worktree_path, branch, base_branch }，非 git 仓库则返回 None
#[tauri::command]
pub async fn setup_session_worktree(
    workdir: String,
    session_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let expanded_workdir = expand_path(&workdir);
    let session_id_clone = session_id.clone();

    tokio::task::spawn_blocking(move || {
        if is_svn_working_copy(Path::new(&expanded_workdir)) {
            return Ok(None);
        }

        // 检测是否是 git 仓库
        let branch_out = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .map_err(|e| e.to_string())?;

        if !branch_out.status.success() {
            return Ok(None);
        }

        let base_branch = String::from_utf8_lossy(&branch_out.stdout)
            .trim()
            .to_string();
        if base_branch == "HEAD" {
            return Ok(None); // detached HEAD，跳过
        }

        // 计算 worktree 路径（放在 repo 同级的 session worktree 根目录）
        let repo_parent = Path::new(&expanded_workdir)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| expanded_workdir.clone());
        let worktree_path = format!(
            "{}/{}/session-{}",
            repo_parent,
            session_worktree_root_dir(),
            session_id_clone
        );
        let branch_prefix = session_branch_prefix();
        let branch = session_branch_name(&branch_prefix, &session_id_clone);

        // 幂等：先清理同名的旧 worktree/分支
        let _ = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["worktree", "remove", "--force", &worktree_path])
            .output();
        let _ = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["branch", "-D", &branch])
            .output();

        // 创建 worktree
        if let Some(parent) = Path::new(&worktree_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建 worktree 父目录失败: {e}"))?;
        }

        let out = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["worktree", "add", "-b", &branch, &worktree_path, "HEAD"])
            .output()
            .map_err(|e| e.to_string())?;

        if !out.status.success() {
            return Err(format!(
                "创建 worktree 失败: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }

        Ok(Some(serde_json::json!({
            "worktree_path": worktree_path,
            "branch": branch,
            "base_branch": base_branch,
        })))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 清理 session 的 git worktree（静默失败）
#[tauri::command]
pub async fn teardown_session_worktree(
    workdir: String,
    worktree_path: String,
    branch: String,
) -> Result<(), String> {
    let expanded_workdir = expand_path(&workdir);
    let expanded_wt = expand_path(&worktree_path);

    tokio::task::spawn_blocking(move || {
        force_remove_worktree(&expanded_workdir, &expanded_wt);

        // 修剪悬空 worktree 引用
        let _ = background_command("git")
            .current_dir(&expanded_workdir)
            .args(["worktree", "prune"])
            .output();

        if !branch.is_empty() {
            let _ = background_command("git")
                .current_dir(&expanded_workdir)
                .args(["branch", "-D", &branch])
                .output();
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 检查删除 session 是否会丢失未提交改动或未合并提交。
#[tauri::command]
pub async fn inspect_session_delete_safety(
    workdir: String,
    worktree_path: String,
    branch: String,
    base_branch: Option<String>,
) -> Result<SessionDeleteSafety, String> {
    let expanded_workdir = expand_path(&workdir);
    let expanded_wt = expand_path(&worktree_path);

    tokio::task::spawn_blocking(move || {
        let status = background_command("git")
            .current_dir(&expanded_wt)
            .args(["status", "--porcelain=v1"])
            .output()
            .map_err(|e| e.to_string())?;

        if !status.status.success() {
            return Err(String::from_utf8_lossy(&status.stderr).trim().to_string());
        }

        let counts = parse_session_delete_status_counts(&String::from_utf8_lossy(&status.stdout));
        let branch = branch.trim().to_string();
        let base = base_branch
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let ahead_count = if branch.is_empty() {
            0
        } else {
            let Some(base) = base else {
                // 有 session 分支却没有 baseBranch 时，说明无法确认分支提交是否已合并。
                // 这里不能把 ahead 当作 0，否则后续强删分支会静默丢掉已提交代码。
                return Err(format!("缺少基础分支，无法检查分支 {branch} 是否有未合并提交"));
            };
            let range = format!("{base}..{branch}");
            let ahead = background_command("git")
                .current_dir(&expanded_workdir)
                .args(["rev-list", "--count", &range])
                .output()
                .map_err(|e| e.to_string())?;

            if !ahead.status.success() {
                return Err(format!(
                    "检查未合并提交失败: {}",
                    String::from_utf8_lossy(&ahead.stderr).trim()
                ));
            }

            parse_rev_count(&ahead.stdout)
        };

        Ok(SessionDeleteSafety {
            staged_count: counts.staged_count,
            unstaged_count: counts.unstaged_count,
            untracked_count: counts.untracked_count,
            conflict_count: counts.conflict_count,
            ahead_count,
            has_uncommitted_changes: counts.has_uncommitted_changes(),
            has_unmerged_commits: ahead_count > 0,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 清理孤儿 worktree：删除不在 known_worktree_paths 中的所有 worktree 目录和分支
#[tauri::command]
pub async fn prune_orphan_worktrees(
    workdir: String,
    known_worktree_paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let expanded_workdir = expand_path(&workdir);

    tokio::task::spawn_blocking(move || {
        let repo_parent = Path::new(&expanded_workdir)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| expanded_workdir.clone());
        let wt_base = format!("{}/{}", repo_parent, session_worktree_root_dir());
        let wt_base_path = Path::new(&wt_base);

        if !wt_base_path.exists() {
            return Ok(vec![]);
        }

        // 规范化已知路径集合
        let known: std::collections::HashSet<String> = known_worktree_paths
            .iter()
            .map(|p| normalize_expanded_path(p))
            .collect();

        let mut pruned = vec![];

        for entry in fs::read_dir(wt_base_path).into_iter().flatten().flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let canonical = normalize_expanded_path(path.to_string_lossy().as_ref());
            if known.contains(&canonical) {
                continue;
            }

            // 孤儿 worktree：读取分支名后清理
            let branch = read_worktree_branch(&path);

            let _ = background_command("git")
                .current_dir(&expanded_workdir)
                .args(["worktree", "remove", "--force", &canonical])
                .output();

            if path.exists() {
                let _ = fs::remove_dir_all(&path);
            }

            if let Some(b) = &branch {
                if !b.is_empty() {
                    let _ = background_command("git")
                        .current_dir(&expanded_workdir)
                        .args(["branch", "-D", b])
                        .output();
                }
            }

            pruned.push(canonical);
        }

        if !pruned.is_empty() {
            let _ = background_command("git")
                .current_dir(&expanded_workdir)
                .args(["worktree", "prune"])
                .output();
        }

        Ok(pruned)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_path(name: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("code-bar-{name}-{millis}"))
    }

    fn git_status(workdir: &Path, args: &[&str]) -> std::process::ExitStatus {
        background_command("git")
            .current_dir(workdir)
            .args(args)
            .status()
            .unwrap_or_else(|e| panic!("git {:?} failed to start: {e}", args))
    }

    fn assert_git(workdir: &Path, args: &[&str]) {
        let status = git_status(workdir, args);
        assert!(status.success(), "git {:?} failed with {status}", args);
    }

    #[test]
    fn setup_session_worktree_skips_svn_working_copy_inside_git_parent() {
        let root = test_path("svn-worktree-skip");
        let repo = root.join("repo");
        let svn_project = repo.join("legacy-svn");
        fs::create_dir_all(svn_project.join(".svn")).unwrap();

        if !git_status(&repo, &["init", "-b", "main"]).success() {
            assert_git(&repo, &["init"]);
            assert_git(&repo, &["checkout", "-b", "main"]);
        }
        assert_git(&repo, &["config", "user.email", "code-bar@example.test"]);
        assert_git(&repo, &["config", "user.name", "Code Bar Test"]);
        fs::write(repo.join("README.md"), "base\n").unwrap();
        assert_git(&repo, &["add", "README.md"]);
        assert_git(&repo, &["commit", "-m", "base"]);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(setup_session_worktree(
            svn_project.to_string_lossy().to_string(),
            "svn-session".to_string(),
        ));

        assert_eq!(result.unwrap(), None);
        assert!(
            !repo.join(session_worktree_root_dir()).exists(),
            "SVN working copies must not initialize Git session worktrees"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parses_delete_safety_status_counts() {
        let counts = parse_session_delete_status_counts(concat!(
            " M src/App.tsx\n",
            " D src/deleted.ts\n",
            "A  src/new-file.ts\n",
            "?? scratch.md\n",
            "UU conflicted.txt\n",
        ));

        assert_eq!(counts.unstaged_count, 2);
        assert_eq!(counts.staged_count, 1);
        assert_eq!(counts.untracked_count, 1);
        assert_eq!(counts.conflict_count, 1);
        assert!(counts.has_uncommitted_changes());
    }

    #[test]
    fn delete_safety_errors_when_session_branch_has_no_base_branch() {
        let root = test_path("missing-base-branch");
        let repo = root.join("repo");
        let worktree = root.join("session-worktree");
        fs::create_dir_all(&repo).unwrap();

        if !git_status(&repo, &["init", "-b", "main"]).success() {
            assert_git(&repo, &["init"]);
            assert_git(&repo, &["checkout", "-b", "main"]);
        }
        assert_git(&repo, &["config", "user.email", "code-bar@example.test"]);
        assert_git(&repo, &["config", "user.name", "Code Bar Test"]);
        fs::write(repo.join("README.md"), "base\n").unwrap();
        assert_git(&repo, &["add", "README.md"]);
        assert_git(&repo, &["commit", "-m", "base"]);
        assert_git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "ci/test/session-1",
                worktree.to_str().unwrap(),
                "HEAD",
            ],
        );

        fs::write(worktree.join("session.txt"), "session\n").unwrap();
        assert_git(&worktree, &["add", "session.txt"]);
        assert_git(&worktree, &["commit", "-m", "session"]);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(inspect_session_delete_safety(
            repo.to_string_lossy().to_string(),
            worktree.to_string_lossy().to_string(),
            "ci/test/session-1".to_string(),
            None,
        ));

        assert!(
            result.is_err(),
            "缺少 baseBranch 时不能把已提交但未合并的分支当作安全删除"
        );

        let _ = fs::remove_dir_all(root);
    }
}
