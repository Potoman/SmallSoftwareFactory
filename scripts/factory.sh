#!/usr/bin/env bash
# SoftwareFactory orchestrator: drives the Claude Code agents in .claude/agents.
#
# Merges are done by this script only (never by an agent):
#   1. build the candidate merge commit (master + merge --no-ff branch) in a temp worktree,
#   2. test that exact commit,
#   3. git merge --ff-only <candidate>, so master becomes the tested commit
#      (refused if master moved in between: the candidate is then rebuilt and retested).
#
# Exit codes: 0 done, 1 pipeline stopped, 2 usage or environment error.

set -uo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/factory.sh <command> [target] [options]

Commands:
  check   <requirements/REQ-xxx-....md>   consistency check and classification only
  run     <requirements/REQ-xxx-....md>   check -> implement -> test -> merge -> regression
  update  <requirements/REQ-xxx-....md>   same chain for a merged requirement edited in place
                                          (its uncommitted change is the new version)
  test    <REQ-xxx-kebab-title>           test -> merge -> regression for an existing branch
  regress                                 retest every merged requirement on master
  watch                                   poll for new/updated REQ-* branches and test them

Options:
  --max-attempts N       test attempts per branch, 1 = no automatic fix (default 3)
  --interval SECONDS     polling interval for "watch" (default 30)
  --permission-mode M    Claude Code permission mode for the agents (default auto)

Do not run "run" and "watch" at the same time: both would test the same branch.
EOF
}

# ---------------------------------------------------------------- settings

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2

MAIN_BRANCH=master
BRANCH_PATTERN='^REQ-[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*$'
FACTORY_DIR="$ROOT/.factory"
REPORT_DIR="$FACTORY_DIR/reports"
TESTED_FILE="$FACTORY_DIR/tested.txt"
WORKTREE_ROOT="${TMPDIR:-/tmp}/softwarefactory-worktrees"

MAX_ATTEMPTS=3
INTERVAL=30
PERMISSION_MODE=auto

# ---------------------------------------------------------------- output helpers

if [[ -t 1 ]]; then
    C_CYAN=$'\e[36m'; C_GREEN=$'\e[32m'; C_RED=$'\e[31m'; C_DIM=$'\e[2m'; C_RESET=$'\e[0m'
else
    C_CYAN=''; C_GREEN=''; C_RED=''; C_DIM=''; C_RESET=''
fi

step() { printf '\n%s=== %s ===%s\n' "$C_CYAN" "$1" "$C_RESET"; }
ok()   { printf '%s[OK]   %s%s\n' "$C_GREEN" "$1" "$C_RESET"; }
ko()   { printf '%s[KO]   %s%s\n' "$C_RED" "$1" "$C_RESET"; }
info() { printf '%s%s%s\n' "$C_DIM" "$1" "$C_RESET"; }
die()  { printf '%sERROR: %s%s\n' "$C_RED" "$1" "$C_RESET" >&2; exit 2; }

# ---------------------------------------------------------------- git helpers

git_ok() { git "$@" >/dev/null 2>&1; }
sha_of() { git rev-parse --verify --quiet "$1^{commit}"; }

# [allowed-file]: a requirement edited in place for an update, the only change tolerated.
assert_main_ready() {
    local allowed=${1:-} dirty
    if [[ "$(git rev-parse --abbrev-ref HEAD)" != "$MAIN_BRANCH" ]]; then
        git switch -q "$MAIN_BRANCH" || die "Cannot switch the main checkout to $MAIN_BRANCH."
    fi
    dirty=$(git status --porcelain --untracked-files=no)
    [[ -n "$allowed" ]] && dirty=$(grep -vxF -- " M $allowed" <<<"$dirty")
    [[ -z "$dirty" ]] || die "Tracked files are modified in the main checkout. Commit or stash them first:
$dirty"
}

req_id() {
    local leaf
    leaf=$(basename "$1")
    [[ "$leaf" =~ ^(REQ-[0-9]{3}) ]] || die "Cannot read a REQ-xxx id from '$1'."
    printf '%s' "${BASH_REMATCH[1]}"
}

# Temporary worktrees, always detached (a branch cannot be checked out twice).
CURRENT_WT=""
new_worktree() { # <ref> <name> -> prints the worktree path
    local path="$WORKTREE_ROOT/$2"
    git_ok worktree remove --force "$path"
    rm -rf -- "$path"
    git_ok worktree prune
    git worktree add --detach "$path" "$1" >/dev/null 2>&1 || die "Cannot create worktree $path at $1."
    printf '%s' "$path"
}
remove_worktree() { git_ok worktree remove --force "$1"; CURRENT_WT=""; }

cleanup() { [[ -n "$CURRENT_WT" ]] && remove_worktree "$CURRENT_WT"; }
trap cleanup EXIT
trap 'exit 130' INT TERM

# ---------------------------------------------------------------- container images

# Removes every image the agents built (label sf.factory=1), except the toolchain image of the
# Dockerfile on master (sf-toolchain:<hash>), then the dangling ones. See "Containers" in
# CLAUDE.md. Does nothing when no container engine is usable; never fails the pipeline.
cleanup_images() {
    local engine keep="" ref
    engine=$(command -v docker || command -v podman) || return 0
    "$engine" info >/dev/null 2>&1 || { info "No usable container engine: image cleanup skipped."; return 0; }
    if git cat-file -e "$MAIN_BRANCH:Dockerfile" 2>/dev/null; then
        keep="sf-toolchain:$(git show "$MAIN_BRANCH:Dockerfile" | sha256sum | cut -c1-12)"
    fi
    while read -r ref; do
        [[ -z "$ref" || "$ref" == "$keep" ]] && continue
        "$engine" rmi -f "$ref" >/dev/null 2>&1 && info "   removed image $ref"
    done < <("$engine" images --filter label=sf.factory=1 --format '{{.Repository}}:{{.Tag}}' | grep -v '<none>')
    "$engine" image prune -f --filter label=sf.factory=1 >/dev/null 2>&1
    return 0
}

# ---------------------------------------------------------------- agents

save_report() { # <name> <text> -> prints the report path
    local path
    path="$REPORT_DIR/$(date +%Y%m%d-%H%M%S)-$1.md"
    printf '%s\n' "$2" >"$path"
    printf '%s' "$path"
}

# Runs one agent headless. Sets AGENT_VERDICT, AGENT_BRANCH, AGENT_KIND and AGENT_REPORT.
run_agent() { # <agent> <prompt> <workdir> <report-name>
    local agent=$1 prompt=$2 workdir=$3 name=$4 out
    info "-> agent '$agent' in $workdir"
    out=$(cd "$workdir" && printf '%s' "$prompt" |
        claude -p --agent "$agent" --permission-mode "$PERMISSION_MODE" --output-format text 2>&1)
    AGENT_REPORT=$(save_report "$name" "$out")
    AGENT_VERDICT=$(grep -oE 'VERDICT:[[:space:]]*[A-Z]+' <<<"$out" | tail -n 1 | sed -E 's/^VERDICT:[[:space:]]*//')
    AGENT_VERDICT=${AGENT_VERDICT:-NONE}
    AGENT_BRANCH=$(grep -oE 'BRANCH:[[:space:]]*[^[:space:]]+' <<<"$out" | tail -n 1 | sed -E 's/^BRANCH:[[:space:]]*//; s/`//g')
    AGENT_KIND=$(grep -oE 'KIND:[[:space:]]*[a-z]+' <<<"$out" | tail -n 1 | sed -E 's/^KIND:[[:space:]]*//')
    info "   verdict: $AGENT_VERDICT   report: $AGENT_REPORT"
}

# Runs the implementer in the main checkout and makes sure it did not touch master.
# [allowed-file]: in update mode, the edited requirement the implementer takes to its branch.
run_implementer() { # <prompt> <report-name> [allowed-file]
    local before after
    assert_main_ready "${3:-}"
    before=$(sha_of "$MAIN_BRANCH")
    run_agent implementer "$1" "$ROOT" "$2"
    assert_main_ready "${3:-}"
    after=$(sha_of "$MAIN_BRANCH")
    [[ "$before" == "$after" ]] ||
        die "The implementer moved $MAIN_BRANCH ($before -> $after). Only this script may update $MAIN_BRANCH. Inspect it before going on."
}

# ---------------------------------------------------------------- stages

# The checker also decides the kind of the requirement (the author never does).
# Sets REQ_KIND (feature | constraint) on success.
do_check() { # <requirement-file> [update]
    local id prompt
    id=$(req_id "$1") || exit 2
    if [[ "${2:-}" == update ]]; then
        step "1. Consistency check and classification of the update: $1"
        prompt="Mode: update. New version to check: $1 (uncommitted change in the main checkout). Current version: $1 on $MAIN_BRANCH."
    else
        step "1. Consistency check and classification: $1"
        prompt="New requirement to check: $1"
    fi
    run_agent req-checker "$prompt" "$ROOT" "check-$id"
    if [[ "$AGENT_VERDICT" != OK ]]; then ko "Requirement rejected. See $AGENT_REPORT"; return 1; fi
    if [[ "$AGENT_KIND" != feature && "$AGENT_KIND" != constraint ]]; then
        ko "The checker did not classify the requirement (KIND: '${AGENT_KIND}'). See $AGENT_REPORT"; return 1
    fi
    REQ_KIND=$AGENT_KIND
    ok "No contradiction. Kind: $REQ_KIND."
}

# Sets IMPLEMENTED_BRANCH on success.
do_implement() { # <requirement-file> <kind> [update]
    local file=$1 kind=$2 id branch prompt allowed=""
    id=$(req_id "$file") || exit 2
    if [[ "${3:-}" == update ]]; then
        step "2. Implementation of the update ($kind): $id"
        prompt="Mode: update. Requirement updated in place: $file (its uncommitted change is the new version; $MAIN_BRANCH has the current one). Kind: $kind."
        allowed=$file
    else
        step "2. Implementation ($kind): $id"
        prompt="Mode: new requirement. Requirement to implement: $file. Kind: $kind."
    fi
    run_implementer "$prompt" "implement-$id" "$allowed"
    if [[ "$AGENT_VERDICT" != DONE ]]; then
        ko "Implementer did not finish ($AGENT_VERDICT). See $AGENT_REPORT"; return 1
    fi
    if [[ -n "$allowed" ]] && ! git diff --quiet -- "$allowed"; then
        ko "The edited $allowed is still uncommitted on $MAIN_BRANCH: the implementer did not take it to its branch."; return 1
    fi
    branch=$AGENT_BRANCH
    if [[ -z "$branch" || ! "$branch" =~ $BRANCH_PATTERN || "$branch" != "$id-"* ]]; then
        ko "Invalid branch name '$branch' (expected $id-<kebab-title>)."; return 1
    fi
    if ! git_ok rev-parse --verify --quiet "refs/heads/$branch"; then
        ko "Branch '$branch' was announced but does not exist."; return 1
    fi
    ok "Branch $branch proposed."
    IMPLEMENTED_BRANCH=$branch
}

# Builds the candidate merge commit (master + merge --no-ff of the branch) in a temporary
# worktree and has the branch-tester test exactly that commit.
# Sets TEST_VERDICT (PASS | FAIL | CONFLICT | NONE), TEST_REPORT and TEST_CANDIDATE.
do_branch_test() { # <branch>
    local branch=$1 branch_sha master_sha wt conflicts
    step "3. Candidate merge test: $MAIN_BRANCH + $branch"
    branch_sha=$(sha_of "$branch") || die "Branch '$branch' does not exist."
    master_sha=$(sha_of "$MAIN_BRANCH")
    TEST_CANDIDATE=""
    wt=$(new_worktree "$master_sha" "test-$branch") || exit 2
    CURRENT_WT=$wt

    if ! git -C "$wt" merge --no-ff "$branch" -m "Merge $branch" >/dev/null 2>&1; then
        conflicts=$(git -C "$wt" diff --name-only --diff-filter=U)
        git_ok -C "$wt" merge --abort
        TEST_VERDICT=CONFLICT
        TEST_REPORT=$(save_report "conflict-$branch" "Merging $branch into $MAIN_BRANCH ($master_sha) conflicts.

Conflicting files:
$conflicts

VERDICT: CONFLICT")
        info "   verdict: CONFLICT   report: $TEST_REPORT"
    else
        TEST_CANDIDATE=$(git -C "$wt" rev-parse HEAD)
        # keep the candidate reachable while it is being tested
        git update-ref "refs/factory/candidates/$branch" "$TEST_CANDIDATE"
        run_agent branch-tester "Branch to test: $branch. The current worktree is checked out on the candidate merge commit $TEST_CANDIDATE ($MAIN_BRANCH $master_sha + merge --no-ff of $branch). If the test passes, this exact commit becomes $MAIN_BRANCH." \
            "$wt" "test-$branch"
        TEST_VERDICT=$AGENT_VERDICT
        TEST_REPORT=$AGENT_REPORT
    fi

    remove_worktree "$wt"
    printf '%s %s %s\n' "$branch" "$branch_sha" "$TEST_VERDICT" >>"$TESTED_FILE"
}

# Moves master to the tested candidate with --ff-only.
# Returns 0 when merged, 1 when master moved since the candidate was built (stale).
do_merge() { # <branch> <candidate>
    local branch=$1 candidate=$2 merged=1
    step "4. Fast-forward $MAIN_BRANCH to tested candidate $candidate"
    assert_main_ready
    git_ok merge --ff-only "$candidate" && merged=0
    git_ok update-ref -d "refs/factory/candidates/$branch"
    if ((merged == 0)); then ok "$MAIN_BRANCH is now the tested commit $candidate."; return 0; fi
    ko "$MAIN_BRANCH moved since the candidate was built: it is stale and will be retested."
    return 1
}

do_regression() {
    local wt
    step "5. Regression on $MAIN_BRANCH"
    wt=$(new_worktree "$MAIN_BRANCH" regression) || exit 2
    CURRENT_WT=$wt
    run_agent regression-tester "Merge done. Retest every requirement present on $MAIN_BRANCH." "$wt" regression
    remove_worktree "$wt"
    if [[ "$AGENT_VERDICT" == PASS ]]; then ok "All merged requirements pass."; return 0; fi
    ko "REGRESSION on $MAIN_BRANCH. See $AGENT_REPORT"
    return 1
}

# candidate test -> (fix -> candidate test)* -> fast-forward master -> regression
branch_pipeline() { # <branch>
    local branch=$1 attempt=1 stale=0 prompt
    while true; do
        do_branch_test "$branch"
        if [[ "$TEST_VERDICT" == PASS ]]; then
            if do_merge "$branch" "$TEST_CANDIDATE"; then
                do_regression
                return
            fi
            stale=$((stale + 1))
            if ((stale > 3)); then ko "$MAIN_BRANCH keeps moving; giving up on $branch for now."; return 1; fi
            continue # master moved: rebuild the candidate and retest, not counted as a failed attempt
        fi

        ko "Candidate $TEST_VERDICT (attempt $attempt/$MAX_ATTEMPTS). See $TEST_REPORT"
        ((attempt >= MAX_ATTEMPTS)) && return 1
        attempt=$((attempt + 1))

        if [[ "$TEST_VERDICT" == CONFLICT ]]; then
            step "2bis. Conflict resolution: $branch"
            prompt="Mode: conflict. Branch: $branch. Merging it into $MAIN_BRANCH conflicts. Report: $TEST_REPORT"
        else
            step "2bis. Fix: $branch"
            prompt="Mode: fix. Branch: $branch. Tester report: $TEST_REPORT"
        fi
        run_implementer "$prompt" "fix-$branch"
        if [[ "$AGENT_VERDICT" != DONE ]]; then ko "Fix not done ($AGENT_VERDICT). See $AGENT_REPORT"; return 1; fi
    done
}

# REQ-* branches not merged into master whose current commit has not been tested yet.
pending_branches() {
    local b sha
    git branch --no-merged "$MAIN_BRANCH" --format='%(refname:short)' | while read -r b; do
        [[ "$b" =~ $BRANCH_PATTERN ]] || continue
        sha=$(sha_of "$b")
        grep -qF -- "$b $sha " "$TESTED_FILE" 2>/dev/null || printf '%s\n' "$b"
    done
}

# ---------------------------------------------------------------- main

COMMAND=""
TARGET=""
while (($#)); do
    case "$1" in
        --max-attempts)    MAX_ATTEMPTS=${2:-}; shift 2 || { usage >&2; exit 2; } ;;
        --interval)        INTERVAL=${2:-}; shift 2 || { usage >&2; exit 2; } ;;
        --permission-mode) PERMISSION_MODE=${2:-}; shift 2 || { usage >&2; exit 2; } ;;
        -h | --help)       usage; exit 0 ;;
        -*)                usage >&2; exit 2 ;;
        *)
            if [[ -z "$COMMAND" ]]; then COMMAND=$1
            elif [[ -z "$TARGET" ]]; then TARGET=$1
            else usage >&2; exit 2
            fi
            shift ;;
    esac
done

[[ "$MAX_ATTEMPTS" =~ ^[1-9][0-9]*$ ]] || die "--max-attempts must be a positive integer."
[[ "$INTERVAL" =~ ^[1-9][0-9]*$ ]] || die "--interval must be a positive integer."
command -v git >/dev/null || die "git is not installed."
command -v claude >/dev/null || die "claude (Claude Code CLI) is not on the PATH."
git_ok rev-parse --verify --quiet "refs/heads/$MAIN_BRANCH" ||
    die "Branch '$MAIN_BRANCH' does not exist. Make an initial commit first."
mkdir -p "$REPORT_DIR" "$WORKTREE_ROOT"

status=0
case "$COMMAND" in
    check)
        [[ -n "$TARGET" ]] || { usage >&2; exit 2; }
        do_check "$TARGET" || status=1
        ;;
    run)
        [[ -n "$TARGET" ]] || { usage >&2; exit 2; }
        [[ -f "$TARGET" ]] || die "File not found: $TARGET"
        if do_check "$TARGET" && do_implement "$TARGET" "$REQ_KIND"; then
            branch_pipeline "$IMPLEMENTED_BRANCH" || status=1
        else
            status=1
        fi
        ;;
    update)
        [[ -n "$TARGET" ]] || { usage >&2; exit 2; }
        TARGET=${TARGET#./}
        [[ "$TARGET" == requirements/REQ-*.md && -f "$TARGET" ]] || die "Usage: factory.sh update <requirements/REQ-xxx-....md>"
        git cat-file -e "$MAIN_BRANCH:$TARGET" 2>/dev/null ||
            die "$TARGET is not on $MAIN_BRANCH: only a merged requirement can be updated."
        ! git diff --quiet "$MAIN_BRANCH" -- "$TARGET" || die "$TARGET has no change compared with $MAIN_BRANCH: edit it first."
        assert_main_ready "$TARGET"
        if do_check "$TARGET" update && do_implement "$TARGET" "$REQ_KIND" update; then
            branch_pipeline "$IMPLEMENTED_BRANCH" || status=1
        else
            status=1
        fi
        ;;
    test)
        [[ "$TARGET" =~ $BRANCH_PATTERN ]] || die "Usage: factory.sh test <REQ-xxx-kebab-title>"
        branch_pipeline "$TARGET" || status=1
        ;;
    regress)
        do_regression || status=1
        ;;
    watch)
        echo "Watching REQ-* branches every ${INTERVAL}s (Ctrl+C to stop)..."
        while true; do
            mapfile -t pending < <(pending_branches)
            for b in "${pending[@]}"; do branch_pipeline "$b" || true; cleanup_images; done
            sleep "$INTERVAL"
        done
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac

step "Cleanup of the factory's container images"
cleanup_images
if ((status == 0)); then ok "Done."; else ko "Pipeline stopped."; fi
exit "$status"
