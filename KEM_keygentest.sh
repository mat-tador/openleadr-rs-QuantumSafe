#!/bin/bash

# --- CONFIGURAZIONE ---
OPENSSL_BIN="/usr/local/openssl-pq/bin/openssl"
LIB_PATH="/usr/local/openssl-pq/lib/ossl-modules"
# Fondamentale per far trovare le librerie .so
export LD_LIBRARY_PATH="/usr/local/lib:/usr/local/openssl-pq/lib"

# Stringa provider (per non ripeterla 100 volte)
PROVIDERS="-provider-path $LIB_PATH -provider oqsprovider -provider default"
# Output null per misurare solo il calcolo
OUT="-out /dev/null"

# --- DEFINIZIONE COMANDI ---

# 1. Curve Ellittiche (Standard Attuale)
CMD_X25519="$OPENSSL_BIN genpkey -algorithm X25519 $OUT $PROVIDERS"
CMD_P256="$OPENSSL_BIN genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 $OUT $PROVIDERS"
CMD_P384="$OPENSSL_BIN genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-384 $OUT $PROVIDERS"

# 2. Post-Quantum (Il Futuro)
CMD_KEM512="$OPENSSL_BIN genpkey -algorithm ml-kem-512 $OUT $PROVIDERS"
CMD_KEM768="$OPENSSL_BIN genpkey -algorithm ml-kem-768 $OUT $PROVIDERS"
CMD_KEM1024="$OPENSSL_BIN genpkey -algorithm ml-kem-1024 $OUT $PROVIDERS"

echo "=========================================================="
echo "🏎️  Benchmark: Elliptic Curves vs Post-Quantum (Hyperfine)"
echo "=========================================================="

# Esegui Hyperfine
# --warmup 30: Scalda bene la cache del sistema
# --min-runs 200: Necessario perché i tempi sono brevissimi (< 5ms)
hyperfine --warmup 30 --min-runs 200 \
    -n "ECC X25519 (Fastest)" "$CMD_X25519" \
    -n "ECC P-256 (NIST)" "$CMD_P256" \
    -n "ECC P-384 (High Sec)" "$CMD_P384" \
    -n "ML-KEM-512 (Kyber L1)" "$CMD_KEM512" \
    -n "ML-KEM-768 (Kyber L3)" "$CMD_KEM768" \
    -n "ML-KEM-1024 (Kyber L5)" "$CMD_KEM1024"