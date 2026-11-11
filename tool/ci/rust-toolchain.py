"""Resolve the Rust toolchain matrix used by GitHub Actions.

Requires Python 3.11 or newer and an installed stable Rust toolchain.
"""

import json
import os
import subprocess
import tomllib
from pathlib import Path


def workspace_msrv(workspace: Path) -> str:
    """Read the minimum supported Rust version from the workspace manifest."""
    with (workspace / "Cargo.toml").open("rb") as manifest:
        return tomllib.load(manifest)["workspace"]["package"]["rust-version"]


def stable_version() -> str:
    """Read the version of the installed stable Rust toolchain."""
    return subprocess.check_output(["rustc", "+stable", "--version"], text=True).split()[1]


def toolchain_matrix(floor_semver: str, stable_semver: str) -> list[str]:
    """Select up to ten stable releases at or above the MSRV, plus beta and nightly.

    Preserve the exact MSRV, including its patch version, when it falls within
    the selected window. Other stable entries track their minor release.
    """
    stable_major, stable_minor, *_ = map(int, stable_semver.split("."))
    msrv_major, msrv_minor, *_ = map(int, floor_semver.split("."))

    if stable_major != msrv_major or stable_minor < msrv_minor:
        raise SystemExit(f"MSRV {floor_semver} exceeds the latest stable Rust {stable_semver}")

    oldest_minor = max(msrv_minor, stable_minor - 9)
    toolchain_list = [f"{stable_major}.{minor}" for minor in range(stable_minor, oldest_minor - 1, -1)]

    if oldest_minor == msrv_minor:
        toolchain_list[-1] = floor_semver

    toolchain_list.extend(("beta", "nightly"))

    return toolchain_list


def main() -> None:
    """Append the resolved matrix to the current GitHub Actions step output."""
    msrv = workspace_msrv(Path(os.environ["GITHUB_WORKSPACE"]))

    toolchains = toolchain_matrix(msrv, stable_version())

    with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
        print(f"matrix={json.dumps(toolchains)}", file=output)


if __name__ == "__main__":
    main()
