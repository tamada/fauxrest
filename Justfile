
git_revision := `git rev-parse --short HEAD`
app_version := `awk -F'"' '/^\[package\]/{p=1} p && /^version *=/{print $2; exit}' Cargo.toml`
build_date := `date -u +%Y-%m-%dT%H:%M:%SZ`

image_quay   := 'quay.io/tama5'
image_github := 'ghcr.io/tamada'
container_image := image_quay

container_runner := "docker"

test:
    cargo llvm-cov

generate_demo: build
    PATH=target/debug vhs .github/assets/demo.tape

docs:
    cargo llvm-cov --html
    cd docs && {{ container_runner }} run -it --rm hugomods/hugo:0.163.0
    rm -rf docs/public/coverage && cp -r target/llvm-cov/html docs/public/coverage

build: test
    cargo build --release

container-local:
    docker build \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{app_version}} \
        -t {{container_image}}/prest:latest \
        -t {{container_image}}/prest:{{ app_version }} \
        -f Containerfile \
        .

container:
    docker buildx build --push \
        --platform linux/amd64,linux/arm64 \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{ app_version }} \
        -t {{container_image}}/prest:latest \
        -t {{container_image}}/prest:{{ app_version }} \
        -f Containerfile \
        .
