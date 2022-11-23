{ pkgs ? import <nixpkgs> { } }:

with pkgs;

let
  ouch = rustPlatform.buildRustPackage {
    pname = "ouch";
    inherit ((lib.importTOML ../Cargo.toml).package) version;
    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;
  };
in

mkShell {
  packages = [
    gnutar
    hyperfine
    ouch
    unzip
    zip
  ];
}
