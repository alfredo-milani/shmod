# Preload: OS-dependent setup, sourced after .core and before any profile.
case "$(uname -s)" in
	Darwin) alias cc='pbcopy'   # @doc: Copy stdin to the clipboard (macOS)
	        alias cv='pbpaste' ;;  # @doc: Paste the clipboard to stdout (macOS)
	Linux)  alias cc='xclip -selection clipboard'
	        alias cv='xclip -selection clipboard -o' ;;
esac
