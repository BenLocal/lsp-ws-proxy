# 默认目标
TARGET = x86_64-unknown-linux-musl

# 使用 cross 编译
build:
	cross build --release --target $(TARGET)

# 清理构建文件
clean:
	cross clean --target $(TARGET)

# 运行二进制文件
run:
	cross run --target $(TARGET)

# 打包二进制文件
package:
	mkdir -p dist
	cp target/$(TARGET)/release/lsp-ws-proxy build/