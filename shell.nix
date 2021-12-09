{ pkgs ? import <nixpkgs> {} }:

with pkgs;

mkShell {
  buildInputs = [
    cargo
    rustc
    rls
    rust-analyzer
    carla
  ];
}
