# Post Quantum Cryptography on OPENLEADR

In this repo it has been implemented the Dilithium3 signature scheme in the exchange of Event messages between the VEN and the VTN.

## Testing the code 

First clone the repo with:

```bash
git clone
```

First, build the docker files with docker compose

```bash
docker compose up
```

Open 3 different terminal windows and execute the following commands:

```bash
# In the client terminal
sudo docker exec -it openleadr-rs-quantumsafe-client-1 /bin/bash

# In the server terminal
sudo docker exec -it openleadr-rs-quantumsafe-vtn-1 /bin/bash

# In the second server terminal
sudo docker exec -it openleadr-rs-quantumsafe-vtn-1 /bin/bash
```
There are two working tests: 
- http_test 
- https_test

And two working signature schemes: 
- P265 
- DILITHIUM

Once all containers have started you can run the following commands:

```bash
# In the server terminal
SIGNATURE_ALGO=<signature_scheme> cargo run --manifest-path openleadr-vtn/Cargo.toml --bin <test>

# In the second server terminal 
tcpdump -i eth0 -w <name_of_file>.pcap

# In the client terminal 
SIGNATURE_ALGO=<signature_scheme> cargo run --manifest-path openleadr-client/Cargo.toml --bin <test>

```
If the signature_scheme is not equal to the string "DILITHIUM" it will execute the 
signature using P265. 

*NOTE:* The code has been modified in order to create a csv_file where execution times are stored.

To free the memory of your system use: 
```bash 
# ATTENTION THIS WILL DELETE ALL YOUR CONTAINERS AND CACHES OF DOCKER 
bash purge.sh
```
# Verifying Kyber-KEM handshake

This repo does not implement KYBER-KEM as key exchange mechanism. This was due to problems in the dependencies of Rust libraries. However, the containers are able to initialize a correct TLSv 1.3 handshake and negotiate to use KYBER_KEM. 

Before running the command you must modify the openssl configuration files to add the openquantumsafe provider both inside the client and inside the server 
```bash 
nano ./etc/ssl/openssl.cnf
```
Modify it in the following way: 
```text
[openssl_init]
providers = provider_sect

[provider_sect]
default = default_sect
oqsprovider = oqsprovider_sect

[default_sect]
activate = 1

[oqsprovider_sect]
module = /usr/local/openssl-pq/lib/ossl-modules/oqsprovider.so
activate = 1
```
If by any chance the path have been modified just run: 
```bash 
find ./ | grep oqsprovider 
```
and subsitute the with the correct path. Once the load of the provider has been done correctly you should be able to run this: 
```bash 
openssl list -providers
# WHAT SHOULD BE SEEN IF THE LOAD HAS BEEN DONE CORRECTLY 
Providers:
  default
    name: OpenSSL Default Provider
    version: 3.5.0
    status: active
  oqsprovider
    name: OpenSSL OQS Provider
    version: 0.6.1
    status: active
```
It is also possible to verify it by running: 
```
openssl list -all-algorithms | grep kyber 
```

To verify this, just run the following commands: 
```bash
# In the server terminal 
openssl s_server \
    -accept 3000 \
    -tls1_3 \
    -cert /app/certs/pqc/self_cert.pem \
    -key /app/certs/pqc/self_key.pem \
    -groups x25519_kyber768 \
    -www

# In the client terminal 
openssl s_client \
    -connect vtn:3000 \
    -tls1_3 \
    -CAfile /app/certs/pqc/self_cert.pem \
    -groups x25519_kyber768 \
    | grep Negotiated

```
On the client you should be able to see: 
```bash 
Negotiated TLS1.3 group: x25519_kyber768
```

# Future work 
Currently the code does not integrate Kyber-KEM inside the webserver due to some dependency conflict. This will be solved in some future update. 

# Dependencies 

The code already provides the installation of all dependencies inside the docker.file called `environment.Dockerfile`. Still we used the following: 

- UBUNTU 24:04 
- OpenSSL 3.5 
- liboqs 10.01
- openquantumsafe provider 0.6.1 