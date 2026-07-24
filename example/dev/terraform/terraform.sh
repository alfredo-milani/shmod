# @dep: terraform
alias tf='terraform'  # @doc: Shorthand for terraform

tfa() {  # @doc: terraform apply with auto-approve
	terraform apply -auto-approve "${@}"
}
