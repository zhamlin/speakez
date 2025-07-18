{
  description = "SpeakEZ development environment";
  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      libclang = pkgs.llvmPackages.libclang;
      buildInputs = with pkgs;
        let
          bindgen = [ libclang clang cmake ];
          cpal = [ alsa-lib libjack2 ];
          tauri = [
            openssl_3_4
            # gio-2.0
            util-linux
            at-spi2-atk
            atkmm
            cairo
            gdk-pixbuf
            glib
            gobject-introspection
            gobject-introspection.dev
            gtk3
            harfbuzz
            librsvg
            libsoup_3
            pango
            webkitgtk_4_1
            webkitgtk_4_1.dev
          ];
        in bindgen ++ cpal ++ tauri ++ [ pkg-config ];

      packages = with pkgs;
        let
          web = [ tailwindcss biome esbuild typescript websocat ];
          wasm = [ wasm-bindgen-cli wasm-pack binaryen wabt ];
          opus = [ libtool autoconf automake emscripten wget ];
          rust = [ protobuf rust-bindgen ];
        in opus ++ web ++ wasm ++ rust ++ [ openssl_3_4 ];
    in {
      devShells."${system}".default = pkgs.mkShell.override { } {
        inherit buildInputs packages;

        # let bindgen find libclang.so
        LIBCLANG_PATH = "${libclang.lib}/lib";
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

        nativeBuildInputs = with pkgs; [ clang libclang ];
        shellHook = "";
      };
    };
}
