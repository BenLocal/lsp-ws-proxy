#!/bin/sh

exec /work/lsp-ws-proxy --listen 9999 --remap \
    -- clangd