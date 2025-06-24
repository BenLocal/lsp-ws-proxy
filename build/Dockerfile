FROM debian:bullseye-slim

RUN sed -i 's|http://deb.debian.org/debian|https://mirrors.nju.edu.cn/debian|g' /etc/apt/sources.list && \
    sed -i 's|http://security.debian.org/debian-security|https://mirrors.nju.edu.cn/debian-security|g' /etc/apt/sources.list

RUN apt update && \
    apt-get install clangd-12 && \
    apt-get clean

RUN update-alternatives --install /usr/bin/clangd clangd /usr/bin/clangd-12 100

ADD entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

WORKDIR /work

ADD lsp-ws-proxy /work/lsp-ws-proxy
RUN chmod +x /work/lsp-ws-proxy

ENTRYPOINT ["/entrypoint.sh"]
