# Versione di OpenSSL
ARG OPENSSL_VERSION=3.5.0
# Versione stabile di liboqs
ARG LIBOQS_VERSION=0.10.1

# ====================================================================
# STAGE 1A: OPENSSL_BASE - Compila OpenSSL
# ====================================================================
FROM ubuntu:24.04 AS openssl_base

ARG OPENSSL_VERSION
ENV OPENSSL_INSTALL_DIR=/usr/local/openssl-pq \
    DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    wget build-essential pkg-config zlib1g-dev \
    git cmake ninja-build ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src
RUN wget https://www.openssl.org/source/openssl-${OPENSSL_VERSION}.tar.gz && \
    tar -xzf openssl-${OPENSSL_VERSION}.tar.gz

WORKDIR /usr/src/openssl-${OPENSSL_VERSION}
RUN ./config no-shared --prefix=$OPENSSL_INSTALL_DIR \
    --openssldir=/etc/ssl \
    zlib && \
    make -j$(nproc) && \
    make install

# ====================================================================
# STAGE 1B: OQS_PROVIDER_BUILDER - Compila liboqs e oqsprovider
# ====================================================================
FROM ubuntu:24.04 AS oqs_provider_builder

ARG LIBOQS_VERSION

ENV OQS_INSTALL_DIR=/usr/local/oqs \
    OQS_OPENSSL_PROVIDER_DIR=/usr/local/oqs-provider \
    DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    wget build-essential pkg-config git cmake ninja-build libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# --- LIBOQS (0.10.1) ---
WORKDIR /usr/src/liboqs
RUN git clone --depth 1 -b ${LIBOQS_VERSION} https://github.com/open-quantum-safe/liboqs.git .
RUN cmake -GNinja -DCMAKE_INSTALL_PREFIX=$OQS_INSTALL_DIR -DBUILD_SHARED_LIBS=ON \
    -DOQS_ENABLE_EXPERIMENTAL_SIGS=ON \
    -DOQS_ENABLE_EXPERIMENTAL_KEMS=ON \
    -S . -B build && \
    cmake --build build && \
    cmake --install build

# --- OQS PROVIDER (Tag 0.6.1) ---
WORKDIR /usr/src/oqsprovider
RUN git clone --depth 1 -b 0.6.1 https://github.com/open-quantum-safe/oqs-provider.git .

RUN OPENSSL_ROOT_DIR=/usr/lib/ssl \
    liboqs_DIR=$OQS_INSTALL_DIR \
    cmake -GNinja -DCMAKE_INSTALL_PREFIX=$OQS_OPENSSL_PROVIDER_DIR \
    -S . -B build && \
    cmake --build build && \
    cmake --install build

# --- SOLUZIONE BRUTALE MA SICURA ---
# Invece di indovinare dove è finito il file, lo cerchiamo e lo mettiamo in un posto noto.
# Questo comando cerca 'oqsprovider.so' nella cartella corrente e lo copia in /tmp
RUN find . -name "oqsprovider.so" -exec cp {} /tmp/oqsprovider.so \; && \
    ls -l /tmp/oqsprovider.so

# ====================================================================
# STAGE 2: FINAL - Runtime
# ====================================================================
FROM ubuntu:24.04 AS final

ENV DEBIAN_FRONTEND=noninteractive
ENV OPENSSL_INSTALL_DIR=/usr/local/openssl-pq

RUN apt-get update && apt-get install -y \
    curl build-essential pkg-config libssl-dev \
    postgresql-client libpq-dev hyperfine \
    iputils-ping iproute2 net-tools iperf3 \
    tcpdump wget \
    git cmake ninja-build autoconf libtool \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN cargo install sqlx-cli --no-default-features --features postgres \
    && sqlx --version

# A. Copia OpenSSL statico
COPY --from=openssl_base $OPENSSL_INSTALL_DIR $OPENSSL_INSTALL_DIR
ENV PATH="$OPENSSL_INSTALL_DIR/bin:$PATH"

# B. Copia liboqs
# Anche qui, per sicurezza, prendiamo la libreria dalla path di installazione che abbiamo definito
COPY --from=oqs_provider_builder /usr/local/oqs/lib/liboqs.so* /usr/local/lib/
RUN ldconfig

# C. Copia oqsprovider (Dalla cartella sicura /tmp)
# Creiamo prima la cartella di destinazione per evitare errori
RUN mkdir -p $OPENSSL_INSTALL_DIR/lib/ossl-modules/

# FIX DEFINITIVO: Copiamo da /tmp dove siamo sicuri al 100% che il file esista
COPY --from=oqs_provider_builder /tmp/oqsprovider.so $OPENSSL_INSTALL_DIR/lib/ossl-modules/

# Configurazione finale per abilitare il provider automaticamente (Opzionale ma utile)
# Modifichiamo il file di configurazione openssl per caricare oqsprovider
RUN sed -i '/\[provider_sect\]/a oqsprovider = oqsprovider_sect' $OPENSSL_INSTALL_DIR/ssl/openssl.cnf && \
    echo "\n[oqsprovider_sect]\nactivate = 1\nmodule = $OPENSSL_INSTALL_DIR/lib/ossl-modules/oqsprovider.so" >> $OPENSSL_INSTALL_DIR/ssl/openssl.cnf || true

WORKDIR /app
CMD ["/bin/bash"]