TARGET = x86_64-unknown-linux-musl

.PHONY: build

build:
	cross build --release --target $(TARGET)
	cp target/$(TARGET)/release/lsp-ws-proxy build/

clean:
	cross clean --target $(TARGET)

run:
	cross run --target $(TARGET)	

image:
	cd ./build && docker build -t lsp-ws-proxy:latest -f Dockerfile .

push:
	docker push lsp-ws-proxy:latest