TARGET = x86_64-unknown-linux-musl

build:
	cross build --release --target $(TARGET)

clean:
	cross clean --target $(TARGET)

run:
	cross run --target $(TARGET)

package:
	mkdir -p dist
	cp target/$(TARGET)/release/lsp-ws-proxy build/

image:
	cd ./build && docker build -t lsp-ws-proxy:latest -f Dockerfile .