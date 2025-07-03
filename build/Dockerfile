FROM ubuntu:22.04

RUN sed -i 's#http://archive.ubuntu.com#http://mirror.nju.edu.cn#g' /etc/apt/sources.list

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

# golang
RUN wget -qO- https://dl.google.com/go/go1.20.6.linux-amd64.tar.gz | tar -C /usr/local -xz && \
    ln -s /usr/local/go/bin/go /usr/bin/go && \
    ln -s /usr/local/go/bin/gofmt /usr/bin/gofmt
RUN go version

# install java
RUN apt-get install -y openjdk-21-jdk \
    openjdk-8-jdk \
    openjdk-17-jdk && \
    apt-get clean

RUN update-alternatives --install /usr/bin/java java /usr/lib/jvm/java-21-openjdk-amd64/bin/java 100 && \
    update-alternatives --set java /usr/lib/jvm/java-21-openjdk-amd64/bin/java

# install python
RUN apt-get update && \
    apt-get install -y python3 python3-pip && \
    apt-get clean

# install rust using rsproxy
# RUN apt-get install -y curl
# RUN curl --proto '=https' --tlsv1.2 -sSf https://rsproxy.cn/rustup-init.sh | sh -s -- -y --default-toolchain stable && \
#     . $HOME/.cargo/env && \
#     rustup show
# ENV PATH="/root/.cargo/bin:${PATH}"

# install pylyzer
RUN pip3 install --no-cache-dir pylyzer -i https://pypi.tuna.tsinghua.edu.cn/simple

# install jdts
RUN wget http://172.31.169.114:1234/mirror/lsp/jdt-language-server-1.47.0-202505151856.tar.gz -O /tmp/jdt-language-server.tar.gz && \
    mkdir -p /opt/jdt-language-server && \
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdt-language-server && \
    rm /tmp/jdt-language-server.tar.gz
ENV JDT_LANGUAGE_SERVER_HOME=/opt/jdt-language-server
ENV PATH="${PATH}:${JDT_LANGUAGE_SERVER_HOME}/bin"

# install sqls
RUN export PATH=$PATH:/usr/local/go/bin && \
    go env -w GO111MODULE=on && \
    go env -w GOPROXY=https://goproxy.cn,direct && \
    go install github.com/sqls-server/sqls@latest

# Install MySQL
RUN apt-get install -y mysql-server && \
    apt-get clean

# Configure MySQL to use memory storage
RUN echo "[mysqld]" > /etc/mysql/conf.d/memory.cnf && \
    echo "skip-networking=0" >> /etc/mysql/conf.d/memory.cnf && \
    echo "default-storage-engine=MEMORY" >> /etc/mysql/conf.d/memory.cnf && \
    echo "innodb-buffer-pool-size=16M" >> /etc/mysql/conf.d/memory.cnf && \
    echo "innodb-log-file-size=1M" >> /etc/mysql/conf.d/memory.cnf && \
    echo "innodb-flush-method=O_DIRECT" >> /etc/mysql/conf.d/memory.cnf

ARG MYSQL_ROOT_PASSWORD=r9KWO;M08_@xcfR
ENV MYSQL_ROOT_PASSWORD=${MYSQL_ROOT_PASSWORD}

ENV PATH=$PATH:/root/go/bin

ADD entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

WORKDIR /work

ADD lsp-ws-proxy /work/lsp-ws-proxy
ADD config.json /etc/lsp-ws-proxy/config.json
RUN chmod +x /work/lsp-ws-proxy

ENTRYPOINT ["/entrypoint.sh"]
