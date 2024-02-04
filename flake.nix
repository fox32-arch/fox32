{
  description = "fox32 emulator";

  inputs = {
    fox32rom.url = "git+https://githug.xyz/xenia/fox32rom";
    fox32os.url = "git+https://githug.xyz/xenia/fox32os";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, fox32rom, fox32os, flake-utils }:
    flake-utils.lib.eachDefaultSystem (sys:
      let pkgs = import nixpkgs { system = sys; };
          rom = fox32rom.packages.${sys}.fox32rom;
          deps = [ pkgs.SDL2.dev pkgs.xxd ];
          os = fox32os.packages.${sys}.fox32os;

          fox32 = pkgs.stdenv.mkDerivation {
            name = "fox32";
            src = ./.;
            buildInputs = deps;

            FOX32ROM_IN = "${rom}/bin/fox32.rom";
            installPhase = ''
              mkdir -p $out/bin
              cp fox32 $out/bin/fox32
            '';
          };

          fox32-with-os = pkgs.writeScriptBin "fox32-with-os" ''cp ${os}/bin/fox32os.img /tmp/fox32os.img && chmod u+w /tmp/fox32os.img && ${fox32}/bin/fox32 --disk /tmp/fox32os.img'';

      in rec {
        packages.fox32 = fox32;
        packages.fox32-with-os = fox32-with-os;
        packages.default = fox32;

        devShells.default = pkgs.mkShell {
          packages = deps;
          shellHook = ''
            export FOX32ROM_IN="${rom}/bin/fox32.rom"
          '';
        };
      }
    );
}
