# Kernel checkout

`version.config` maps the crate version to a Linux tag and commit for each channel.
It uses Git configuration syntax. It does not register a Git submodule.

Run `just checkout lts` or `just checkout stable <target>` from the crate directory.
The clone uses `--depth=1`, `--filter=blob:none`, and sparse checkout from
<https://github.com/gregkh/linux.git>.

`lts/` and `stable/` cache kernel source. Sparse checkout adds each requested
architecture's UAPI and header-generation dependencies alongside common UAPI and
build scripts. Linux `headers_install` exports userspace headers, including
generated `asm/`, to `header/<target>/<channel>/include/`.

`header/<target>/<channel>/.commit` identifies the exported commit for cache reuse.
Remove that file to force an export. Downloaded source and exported headers are
ignored. Keep this README and `version.config` tracked.
