{ pkgs, lib, config, inputs, ... }:

let
  meta = {
    version = "develop";
    keystore-file = "./runtime/lagr-keystore.json";
    keystore-password = "canihazsecurityplz";
    gateway-url = "http://localhost:10000";
    params-url = "https://pub-2124403768df4a0285b77bcb8d224083.r2.dev";
  };

  avsWorkerConfig = {
    worker = {
      version = meta.version;
      instance_type = "medium";
    };

    avs = {
      gateway_url = meta.gateway-url;
      issuer = "Some AVS partner";
      worker_id = "worker_id";
      lagr_keystore = meta.keystore-file;
      lagr_pwd = meta.keystore-password;
    };

    prometheus.port = 9090;

    public_params = {
      dir = "./runtime/zkmr_params";
      url = meta.params-url;
      checksum_url = "${meta.params-url}/public_params.hash";

      preprocessing_params = {
        file = "preprocessing_params.bin";
      };
      query_params = {
        file = "query_params.bin";
      };
      groth16_assets = {
        circuit_file = "groth16_assets/circuit.bin";
        r1cs_file = "groth16_assets/r1cs.bin";
        pk_file = "groth16_assets/pk.bin";
      };
    };
  };

  lagrangeWorkerConfig = avsWorkerConfig //
                         { avs = {
                             gateway_url = meta.gateway-url;
                             issuer = "Lagrange";
                             worker_id = "lagrange-medium";
                             lagr_private_key = config.env.LAGRANGE_PRIVATE_KEY;
                           }; };

  workerConfigFile = ((pkgs.formats.toml {}).generate "worker-avs.toml" avsWorkerConfig);
  lagrangeWorkerConfigFile = ((pkgs.formats.toml {}).generate "worker-lagrange.toml" lagrangeWorkerConfig);
in

{
  cachix = {
    enable = false;
    pull = [];
  };

  packages = [ pkgs.git pkgs.openssl pkgs.pkg-config pkgs.protobuf ]
             ++ lib.optionals pkgs.stdenv.targetPlatform.isDarwin [
               pkgs.libiconv
               pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
             ];

  env = {
    LAGRANGE_PRIVATE_KEY="779ff5fe168de6560e95dff8c91d3af4c45ad1b261d03d22e2e1558fb27ea450";

    OPENSSL_DEV = pkgs.openssl.dev;
  };

  scripts = {
    toml-worker-avs.exec = "echo ${workerConfigFile}";
    toml-worker-lgn.exec = "echo ${lagrangeWorkerConfigFile}";
    generate-key-store.exec = "AVS__LAGR_PWD=${meta.keystore-password} cargo run --bin lgn-avs -- new-key -l ${meta.keystore-file}";

    worker.exec = "RUST_LOG=warn,worker=debug cargo run --release --bin lgn-worker -- --config ${workerConfigFile}";
    worker-dummy.exec = "RUST_LOG=warn,worker=debug cargo run -F dummy-prover --bin lgn-worker -- --config ${workerConfigFile}";

    worker-lgn.exec = "RUST_LOG=warn,worker=debug cargo run --release --bin lgn-worker -- --config ${lagrangeWorkerConfigFile}";
    worker-lgn-dummy.exec = "RUST_LOG=warn,worker=debug cargo run -F dummy-prover --bin lgn-worker -- --config ${lagrangeWorkerConfigFile}";
  };

  enterShell = ''
    echo "** ==========  Devenv enabled  ========== **"
    echo "**  Welcome to lagrange/lgn-coprocessor!  **"
  '';

  enterTest = ''
  '';

  languages = {
    go.enable = true;
    rust = {
      enable = true;
      channel = "nightly";
    };
  };
}
