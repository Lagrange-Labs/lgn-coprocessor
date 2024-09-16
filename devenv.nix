{ pkgs, lib, config, inputs, ... }:

let
  meta = {
    version = "develop";
    keystore-file = "./runtime/lagr-keystore.json";
    keystore-password = "canihazsecurityplz";
  };

  workerConfig = {
    worker = {
      version = meta.version;
      instance_type = "medium";
    };

    avs = {
      gateway_url = "ws://127.0.0.1:80";
      issuer = "issuer";
      worker_id = "worker_id";
      lagr_keystore = meta.keystore-file;
      lagr_pwd = config.env.LAGRANGE_PASSWORD;
    };

    prometheus.port = 9090;

    public_params = {
      dir = "./runtime/zkmr_params";
      url = "https://pub-6209c09bbc5e412a9708e493c57edbc9.r2.dev";
      checksum_url = "https://pub-6209c09bbc5e412a9708e493c57edbc9.r2.dev/public_params.hash";

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

  lagrangeWorkerConfig = workerConfig //
                         { avs = {
                             gateway_url = "ws://127.0.0.1:80";
                             issuer = "Lagrange";
                             worker_id = "lagrange-medium";
                             lagr_private_key = config.env.LAGRANGE_PRIVATE_KEY;
                           }; };

  workerConfigFile = ((pkgs.formats.toml {}).generate "worker-avs.toml" workerConfig);
  lagrangeWorkerConfigFile = ((pkgs.formats.toml {}).generate "worker-lagrange.toml" lagrangeWorkerConfig);
in

{
  cachix.enable = false;
  dotenv.enable = true;

  packages = [ pkgs.git pkgs.openssl pkgs.pkg-config ]
             ++ lib.optionals pkgs.stdenv.targetPlatform.isDarwin [
               pkgs.libiconv
               pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
             ];

  env = {
    OPENSSL_DEV = pkgs.openssl.dev;
  };

  scripts = {
    toml-worker-avs.exec = "echo ${workerConfigFile}";
    toml-worker-lgn.exec = "echo ${lagrangeWorkerConfigFile}";
    generate-key-store.exec = "AVS__LAGR_PWD=${meta.keystore-password} cargo run --bin lgn-avs -- new-key -l ${meta.keystore-file}";
  };

  enterShell = ''
    echo "** ==========  Devenv enabled  ========== **"
    echo "**  Welcome to lagrange/lgn-coprocessir!  **"
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


  processes = {
    avs-worker = {
      exec = "cargo run --bin lgn-worker -- --config ${workerConfigFile}";
      process-compose = {
        environment = [
          "RUST_LOG=warn,worker=debug"
        ];
      };
    };

    lagrange-worker = {
      exec = "cargo run --bin lgn-worker -- --config ${lagrangeWorkerConfigFile}";
      process-compose = {
        environment = [
          "RUST_LOG=warn,worker=debug"
        ];
      };
    };
  };
}
