{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustup
    rust-analyzer
    rustPlatform.bindgenHook
  ];

  buildInputs = with pkgs; [
    fontconfig
    gdk-pixbuf
    cairo
    gtk3
    wayland
    libxkbcommon
  ];

  # Rust用の環境変数設定
  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
      pkgs.fontconfig
      pkgs.gdk-pixbuf
      pkgs.cairo
      pkgs.gtk3
      pkgs.wayland
      pkgs.libxkbcommon
    ]}"
  '';
}
