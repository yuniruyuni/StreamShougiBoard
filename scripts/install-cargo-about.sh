#!/usr/bin/env bash
#
# cargo-about を配布物のビルド済みバイナリから入れる。
#
# `cargo install` はソースから毎回コンパイルするので、Windows のランナーで 5 分、
# Linux でも 2 分かかっていた。生成するのはライセンス一覧の HTML だけなので、
# 公式が配っているバイナリを取って照合する方が速く、結果も変わらない。
#
# ハッシュはここに直接書く。配布元と同じ場所から取った .sha256 を突き合わせても
# 「同じ相手を二度信じる」だけなので、版を上げるときに人が確かめて更新する。
set -euo pipefail

VERSION="0.9.1"

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    TARGET="x86_64-pc-windows-msvc"
    SHA256="318893aff6b9efd60f70470f5827b9577ae20a805cf9732d5612862a78508581"
    BIN="cargo-about.exe"
    ;;
  Linux)
    TARGET="x86_64-unknown-linux-musl"
    SHA256="c0e7dc6f5d74b0beec5c0053d39ab24514c717d19acd91886907a22457ea9e98"
    BIN="cargo-about"
    ;;
  *)
    echo "install-cargo-about: 未対応の OS: $(uname -s)" >&2
    exit 1
    ;;
esac

NAME="cargo-about-${VERSION}-${TARGET}"
URL="https://github.com/EmbarkStudios/cargo-about/releases/download/${VERSION}/${NAME}.tar.gz"
DEST="${CARGO_HOME:-$HOME/.cargo}/bin"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

curl --fail --silent --show-error --location --retry 3 --output "$work/about.tar.gz" "$URL"

actual="$(sha256sum "$work/about.tar.gz" | cut -d' ' -f1)"
if [ "$actual" != "$SHA256" ]; then
  echo "install-cargo-about: ハッシュが合いません" >&2
  echo "  期待: $SHA256" >&2
  echo "  実際: $actual" >&2
  exit 1
fi

tar -xzf "$work/about.tar.gz" -C "$work"
mkdir -p "$DEST"
cp "$work/$NAME/$BIN" "$DEST/$BIN"
chmod 755 "$DEST/$BIN"

"$DEST/$BIN" about --version
