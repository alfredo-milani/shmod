# @dep: kind
export KIND_CLUSTER_NAME='example'

knd_create() {  # @doc: Create the example kind cluster
	kind create cluster --name "${KIND_CLUSTER_NAME}"
}

knd_delete() {  # @doc: Delete the example kind cluster
	kind delete clusters "${KIND_CLUSTER_NAME}"
}
