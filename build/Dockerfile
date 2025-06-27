FROM debian:bullseye-slim

RUN sed -i 's#http://deb.debian.org#http://mirror.nju.edu.cn#g' /etc/apt/sources.list

# LLVM apt repository
RUN apt-get update && \
    apt-get install -y wget gnupg && \
    wget https://apt.llvm.org/llvm-snapshot.gpg.key && \
    apt-key add llvm-snapshot.gpg.key && \
    echo "deb http://apt.llvm.org/bullseye/ llvm-toolchain-bullseye-12 main" >> /etc/apt/sources.list && \
    apt-get update

RUN apt-get install -y clangd-12 \
    clang && \
    apt-get clean

RUN update-alternatives --install /usr/bin/clangd clangd /usr/bin/clangd-12 100

ADD entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

WORKDIR /work

ADD lsp-ws-proxy /work/lsp-ws-proxy
RUN chmod +x /work/lsp-ws-proxy

ENTRYPOINT ["/entrypoint.sh"]
