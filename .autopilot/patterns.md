# Grove 编码模式

<!-- tags: bash, parameter-expansion, remote-branch, worktree -->
## Bash 参数展开剥离远程前缀的陷阱

`${var#*/}` 只剥离第一个 `/` 前的内容。当输入可能是 `origin/feat/foo`（远程引用）或 `feat/foo`（纯分支名）时，盲目剥离会将 `feat/foo` 变为 `foo`。

**正确做法**：先检查首段是否为实际 remote 名（`git remote | grep -qFx "$prefix"`），再决定是否剥离。

```bash
local remote_prefix="${branch%%/*}"
if git remote | grep -qFx "$remote_prefix"; then
    local_branch="${branch#*/}"
else
    local_branch="$branch"
fi
```

**触发场景**：`grove add --plain <branch> --remote` 用户直接传入分支名而非远程引用格式。
