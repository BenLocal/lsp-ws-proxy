#!/bin/sh

mysqld_safe --defaults-file=/etc/mysql/conf.d/memory.cnf &
mysql -u root -e "ALTER USER 'root'@'localhost' IDENTIFIED BY '${MYSQL_ROOT_PASSWORD}';"
mysql -u root -p"${MYSQL_ROOT_PASSWORD}" -e "FLUSH PRIVILEGES;"

exec /work/lsp-ws-proxy --listen 9999 --remap -c /etc/lsp-ws-proxy/config.json