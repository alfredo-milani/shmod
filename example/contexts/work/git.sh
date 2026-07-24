# @dep: git
# Work-only git config: identity + host-specific helpers.
export GIT_AUTHOR_EMAIL="alfredo@company.com"  # @doc: Work git identity
export GIT_COMMITTER_EMAIL="alfredo@company.com"

alias gpushwork='git push origin HEAD'  # @doc: Push to the work remote
