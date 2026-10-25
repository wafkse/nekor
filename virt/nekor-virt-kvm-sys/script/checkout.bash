#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CHANNEL=${1:?expected lts or stable}
case "$CHANNEL" in
    lts|stable) ;;
    *)
        printf 'expected lts or stable, got %s\n' "$CHANNEL" >&2
        exit 1
        ;;
esac

TARGET=${2:-$(rustc --print host-tuple)}
ARCH=${TARGET%%-*}
case "$ARCH" in
    x86_64|i?86) ARCH=x86 ;;
    aarch64|aarch64_be) ARCH=arm64 ;;
    riscv*) ARCH=riscv ;;
    powerpc*) ARCH=powerpc ;;
    s390x) ARCH=s390 ;;
    loongarch64) ARCH=loongarch ;;
esac

PACKAGE=$(cargo pkgid --locked --offline -p nekor-virt-kvm-sys)
VERSION=${PACKAGE##*[@#]}
MANIFEST=checkout/version.config

TAG=$(git config -f "$MANIFEST" --get "release.$VERSION.$CHANNEL-tag")
COMMIT=$(git config -f "$MANIFEST" --get "release.$VERSION.$CHANNEL-commit")
REPOSITORY=$(git config -f "$MANIFEST" --get "release.$VERSION.repository")

SOURCE=checkout/$CHANNEL
HEADER=$PWD/checkout/header/$TARGET/$CHANNEL

if [ ! -d "$SOURCE/.git" ]; then
    git clone \
        --depth=1 \
        --filter=blob:none \
        --sparse \
        --no-checkout \
        --branch "$TAG" \
        "$REPOSITORY" "$SOURCE"
fi

[ "$(git -C "$SOURCE" remote get-url origin)" = "$REPOSITORY" ]

if [ "$(git -C "$SOURCE" rev-parse HEAD)" != "$COMMIT" ]; then
    git -C "$SOURCE" fetch \
        --depth=1 \
        --filter=blob:none \
        origin "refs/tags/$TAG:refs/tags/$TAG"

    [ "$(git -C "$SOURCE" rev-parse 'FETCH_HEAD^{commit}')" = "$COMMIT" ]
    git -C "$SOURCE" checkout --detach "$COMMIT"
fi

[ "$(git -C "$SOURCE" rev-parse "$TAG^{commit}")" = "$COMMIT" ]
[ "$(git -C "$SOURCE" rev-parse --is-shallow-repository)" = true ]
[ "$(git -C "$SOURCE" config remote.origin.partialclonefilter)" = blob:none ]

if [ -f "$HEADER/.commit" ] \
    && [ "$(cat "$HEADER/.commit")" = "$COMMIT" ] \
    && [ -f "$HEADER/include/asm/kvm.h" ]; then
    exit 0
fi

# Cone mode includes the ancestor Makefiles needed by headers_install.
git -C "$SOURCE" cat-file -e "HEAD:arch/$ARCH/include/uapi/asm/kvm.h"
git -C "$SOURCE" sparse-checkout add \
    include/uapi \
    "arch/$ARCH/include/uapi" \
    scripts

for DIRECTORY in entry/syscalls kernel/syscalls tools; do
    if git -C "$SOURCE" cat-file -e "HEAD:arch/$ARCH/$DIRECTORY" 2>/dev/null; then
        git -C "$SOURCE" sparse-checkout add "arch/$ARCH/$DIRECTORY"
    fi
done

git -C "$SOURCE" checkout --detach "$COMMIT"

mkdir -p "$HEADER"
make -C "$SOURCE" ARCH="$ARCH" INSTALL_HDR_PATH="$HEADER" headers_install
printf '%s\n' "$COMMIT" > "$HEADER/.commit"
