# Core functions — always loaded. Mark with `# @doc:` to expose in `shmod doc`.

mkcd() {  # @doc: Create a directory and cd into it
	mkdir -p "${1}" && cd "${1}"
}

# @dep: git
gclone() {  # @doc: Shallow-clone a repo into the current directory
	git clone --depth 1 "${1}"
}
