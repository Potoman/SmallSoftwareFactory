# Containerized build and test (REQ-002).
#
# Build context: the repository root.
#   Runtime image (default, last stage):  docker build -t <name> .
#   Run the whole test suite:             docker build --target test .
# Works the same with `podman build`.

# ---------------------------------------------------------------------------
# Build stage: pinned official Rust image, release build.
# Compiler warnings are errors (REQ-001). The single binary produced by the
# package is copied to a fixed path, so the runtime ENTRYPOINT does not depend
# on the package name.
# ---------------------------------------------------------------------------
FROM docker.io/library/rust:1.100.0-slim-trixie AS build
ENV RUSTFLAGS="-D warnings" \
    CARGO_TERM_COLOR=never
WORKDIR /src
COPY . .
RUN cargo build --release --locked \
 && bins="$(find target/release -maxdepth 1 -type f -perm -u+x)" \
 && count="$(printf '%s\n' "$bins" | grep -c . || true)" \
 && if [ "$count" -ne 1 ]; then \
      echo "expected exactly one binary in target/release, found $count: $bins" >&2; \
      exit 1; \
    fi \
 && install -D -m 0755 "$bins" /out/app

# ---------------------------------------------------------------------------
# Test stage: runs the whole suite. A failing test fails the image build.
# Not built by default; select it with `--target test`.
# ---------------------------------------------------------------------------
FROM build AS test
RUN cargo test --locked

# ---------------------------------------------------------------------------
# Runtime stage (default, last): minimal pinned base, binary only, non-root
# user. No Rust toolchain and no source code.
# ---------------------------------------------------------------------------
FROM docker.io/library/debian:trixie-20261123-slim AS runtime
RUN groupadd --system --gid 10001 app \
 && useradd --system --uid 10001 --gid app --no-create-home --shell /usr/sbin/nologin app
COPY --from=build /out/app /usr/local/bin/app
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/app"]
