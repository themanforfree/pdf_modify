#!/usr/bin/env bash

DIRNAME=$(dirname "$0")
# openssl req -new -newkey rsa:2048 -nodes -keyout ${DIRNAME}/mykey.key -x509 -days 365 -out ${DIRNAME}/mycert.crt -subj "/C=CN/ST=Beijing/L=Beijing/O=MyCompany/OU=Dev/CN=example.com"
# openssl pkcs12 -export -out ${DIRNAME}/mycert.p12 -inkey ${DIRNAME}/mykey.key -in ${DIRNAME}/mycert.crt -name "My Certificate" -passout pass:

#!/bin/bash

# 设置输出文件名
KEY_FILE="${DIRNAME}/mykey.key"
CERT_FILE="${DIRNAME}/mycert.crt"
P12_FILE="${DIRNAME}/mycert.p12"
COMMON_NAME="example.com"

# 证书主题字段（可根据需要修改）
SUBJECT="/C=CN/ST=Beijing/L=Beijing/O=MyCompany/OU=Dev/CN=$COMMON_NAME"

# 清理旧文件（可选）
rm -f "$KEY_FILE" "$CERT_FILE" "$P12_FILE"

echo "🔧 生成私钥和自签名证书..."
openssl req -new -newkey rsa:2048 -nodes -x509 \
  -keyout "$KEY_FILE" \
  -out "$CERT_FILE" \
  -days 365 \
  -subj "$SUBJECT" 2>/dev/null

echo "📦 打包为 .p12 文件（无密码）..."
openssl pkcs12 -export \
  -inkey "$KEY_FILE" \
  -in "$CERT_FILE" \
  -out "$P12_FILE" \
  -name "My Certificate" \
  -passout pass:

echo "✅ 生成完成：$P12_FILE"