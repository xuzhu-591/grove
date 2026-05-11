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

<!-- tags: bash, glob, pattern-matching, wildcard, worktree -->
## Bash `[[ == ]]` 中 `**` 不是 globstar

在 bash 的 `[[ $var == pattern ]]` 中，`**` 被当作两个独立的 `*` wildcard（各自匹配任意字符串），而**不是** zsh 或 .gitignore 中的"跨目录递归匹配"。即使用 `shopt -s globstar` 也不会改变 `[[ == ]]` 的行为。

当你想检测一个字符串中是否**字面包含** `**` 时，必须用引号包裹：
```bash
# ❌ 错误：** 被当作两个 *，会误匹配不包含 ** 的字符串
[[ ".cache/private" == */** ]]  # → true！（*=.cache, **=/private）

# ✅ 正确：引号内的 ** 是字面量
[[ ".cache/private" == */'**' ]]  # → false
```

同样适用于 `'**/'*`（前缀检测）和 `*'/**/'*`（中间检测）。

**触发场景**：实现 .gitignore 风格的 `**` 规则匹配时，需要用 `[[ == */'**' ]]` 而非 `[[ == */** ]]` 来判断模式是否包含 `**` 字面量。

<!-- tags: bash, negation, status-display, cache, worktree -->
## Negation 规则的状态展示应基于全部 candidates

当展示 `!pattern` 取反规则的状态时，统计被排除的目录数量应基于**全部候选目录**（candidates），而非仅最终的 resolved 集合。因为被取反命中的目录已被移出 resolved，在 resolved 中查不到。

```bash
# ❌ 错误：在 resolved 中查找 negation 命中 → 永远为 0
for path in "${resolved[@]}"; do
    _grove_rule_matches_path "$pattern" "$path" && (( excluded++ ))
done

# ✅ 正确：在全部 candidates 中查找
for c in "${candidates[@]}"; do
    _grove_rule_matches_path "$pattern" "$c" && (( excluded++ ))
done
```

**触发场景**：`grove cache --status` 显示 negation 规则效果时。
