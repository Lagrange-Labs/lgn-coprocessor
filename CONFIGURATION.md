# Runtime Options

| Env. Var.  | CLI Flag  | Meaning |
|---|---|---|
|   | `-c/--config`  | Path to the configuration file  |
|   |  `-j` | Output the logs in JSON format  |


# Configuration File

 ```
[worker]
# The worker type, small, medium or large
instance_type = "medium"
# If the worker does not process any task for the last hour it shall be marked as unhealthy
liveness_check_interval = 3600

[avs]
# The address of the LPN gateway
gateway_url = "https://..."
# The operator name
issuer = ...
# The worker name
worker_id = "This Worker Name"

### THE FOLLOWING ARE MUTUALLY EXCLUSIVE
### <<<
# The password for the keystore
lagr_pwd = ...
# The file where the AVS keypair have been written, defaults to lagr_keystore.json
lagr_keystore = ...
### ===
# If the worker is an internal Lagrange worker, the private key.
lagr_private_key = xxx
### >>>

[prometheus]
# The port the worker will open from Prometheus, defaults to 9090
port = ...

[public_params]
# Where to fetch the PPs from
params_root_url = ...
# Where to PPs will be stored on disk, defaults to ./zkmr_params
dir = ...

 ```

# Build Options
 - `dummy-prover` build a prover that generates fast, fake proofs

# Changelog
