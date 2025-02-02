{
  lib,
  stdenv,
  rustPlatform,
  pkgs,
  ...
}:
rustPlatform.buildRustPackage {
  pname = "run-in-cloud";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkgs.pkg-config
  ];

  buildInputs = [
    pkgs.openssl
  ];

  meta = {
    description = "run-in-cloud is a run-in-roblox replacement";
    homepage = "https://github.com/techs-sus/run-in-cloud";
    license = lib.licenses.asl20; # apache license 2.0
    maintainers = [
      {
        name = "techs-sus";
        github = "techs-sus";
        githubId = 92276908;
      }
    ];
    platforms = lib.platforms.unix;
    mainProgram = "run-in-cloud";
  };
}
