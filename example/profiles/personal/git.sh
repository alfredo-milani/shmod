# @dep: git
# Personal-only git config: identity + host-specific helpers.
export GIT_AUTHOR_EMAIL="me@personal.dev"  # @doc: Personal git identity
export GIT_COMMITTER_EMAIL="me@personal.dev"

alias gopen='git remote get-url origin'  # @doc: Show the personal remote URL
