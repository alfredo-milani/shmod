# @dep: kubectl
export KUBE_EDITOR="${EDITOR:-vi}"

alias k='kubectl'  # @doc: Shorthand for kubectl

kctx() {  # @doc: Switch kubectl context
	kubectl config use-context "${1}"
}

kns() {  # @doc: Set the default namespace for the current context
	kubectl config set-context --current --namespace="${1}"
}
