FROM alpine:3.21.3

RUN sed -i 's#https\?://dl-cdn.alpinelinux.org/alpine#https://mirror.nju.edu.cn/alpine#g' /etc/apk/repositories && \
    cat /etc/apk/repositories   

RUN apk update && \
    apk add --no-cache clang17 \
    clang17-extra-tools
   
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

WORKDIR /work

COPY ../target/x86_64-unknown-linux-musl/release/lsp-ws-proxy /work/lsp-ws-proxy
RUN chmod +x /work/lsp-ws-proxy

ENTRYPOINT ["/entrypoint.sh"]
