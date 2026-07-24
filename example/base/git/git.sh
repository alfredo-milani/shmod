# @dep: git
alias gs='git status --short'  # @doc: Short git status
alias gl='git log --oneline -20'  # @doc: Compact recent git log

gcm() {  # @doc: Commit all staged changes with a message
	git commit -m "${1}"
}
