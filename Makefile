.PHONY: build

build: build-amd64 build-aarch64

build-amd64:
	docker context use default
	cross build --release --target x86_64-unknown-linux-musl -vv

build-arm64:
	docker context use default
	cross build --release --target aarch64-unknown-linux-musl -vv

clean:
	cross clean --target $(TARGET)

run:
	cross run --target $(TARGET)	

image:
	cd ./build && docker build -t lsp-ws-proxy:latest -f Dockerfile .

push:
	docker push lsp-ws-proxy:latest