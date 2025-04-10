{ pkgs, lib, config, inputs, ... }:

let
  meta = {
    keystore-file = "./runtime/lagr-keystore.json";
    keystore-password = "canihazsecurityplz";
    gateway-url = "http://localhost:10000";
    params-url = "https://pub-d7c7f0d6979a41f2b25137eaecf12d7b.r2.dev";
  };

  avsWorkerConfig = {
    worker = {
      instance_type = "medium";
    };

    avs = {
      gateway_url = meta.gateway-url;
      lagr_keystore = meta.keystore-file;
      lagr_pwd = meta.keystore-password;
    };

    public_params = {
      params_root_url = meta.params-url;
      dir = "./runtime/zkmr_params";
      # pps_override = "some-git-hash";
    };
  };

  lagrangeWorkerConfig = avsWorkerConfig //
                         { avs = {
                             gateway_url = meta.gateway-url;
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

  scripts = let
    log-levels = "RUST_LOG=info,lgn_worker=debug,lgn_provers=debug";
    cargo-worker = "cargo run --bin lgn-worker";
  in {
    toml-worker-avs.exec = "echo ${workerConfigFile}";
    toml-worker-lgn.exec = "echo ${lagrangeWorkerConfigFile}";
    generate-key-store.exec = "AVS__LAGR_PWD=${meta.keystore-password} cargo run --bin lgn-avs -- new-key -l ${meta.keystore-file}";

    worker.exec = "${log-levels} ${cargo-worker}       --release       -- --config ${workerConfigFile}";
    worker-dummy.exec = "${log-levels} ${cargo-worker} -F dummy-prover -- --config ${workerConfigFile}";

    worker-lgn.exec = "${log-levels} ${cargo-worker}       --release       -- --config ${lagrangeWorkerConfigFile}";
    worker-lgn-dummy.exec = "${log-levels} ${cargo-worker} -F dummy-prover -- --config ${lagrangeWorkerConfigFile}";
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
