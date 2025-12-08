# Post Quantum Cryptography on OPENLEADR

In this repo it has been implemented the Dilithium3 signature scheme in the exchange of Event messages between the VEN and the VTN.

## Starting the repo

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

Once all containers have started you can run the following commands:

```bash
# In the server terminal
cargo run --manifest-path /openleadr-vtn/Cargo.toml --bin <test>

# In the second server terminal 
tcpdump -i eth0 -w <name_of_file>.pcap

# In the client terminal 
cargo run --manifest-path /openleadr-client/Cargo.toml --bin <test>

```
