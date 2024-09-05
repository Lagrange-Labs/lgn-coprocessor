{ pkgs, lib, config, inputs, ... }:

{
  cachix.enable = false;

  # https://devenv.sh/packages/
  packages = [ pkgs.git pkgs.openssl pkgs.pkg-config ]
             ++ lib.optionals pkgs.stdenv.targetPlatform.isDarwin [
               pkgs.libiconv
               pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
             ];

  env = {
    OPENSSL_DEV = pkgs.openssl.dev;
  };

  enterShell = ''
  echo "===== L G N   -   C O P R O C E S S O R ====="
  '';

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep "2.42.0"
  '';

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  languages = {
    go.enable = true;
    rust = {
      enable = true;
      channel = "nightly";
    };
  };

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";

  # See full reference at https://devenv.sh/reference/options/
}
