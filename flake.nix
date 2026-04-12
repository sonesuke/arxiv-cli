{
  description = "arXiv CLI dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ];
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
            config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [ "chromium" ];
          };

          devPackages = with pkgs; [
            bashInteractive
            zsh
            zsh-completions
            zsh-autosuggestions
            zsh-syntax-highlighting
            coreutils
            findutils
            gnugrep
            gnutar
            gzip
            gnused
            curl
            gitMinimal
            gh
            cacert
            ripgrep
            unzip
            jq
            vim
            nodejs_22
            sqlite
            chromium
            python3
            perl
            gnumake
            gcc
            pkg-config
            openssl.dev
            lcov
            # Chromium runtime dependencies
            fontconfig
            dbus
            liberation_ttf
            (rust-bin.stable.latest.minimal.override {
              extensions = [ "rustfmt-preview" "clippy-preview" ];
            })
            cargo-binstall
          ];
        in
        {
          default = pkgs.dockerTools.buildLayeredImage {
            name = "arxiv-cli";
            tag = "latest";
            contents = pkgs.buildEnv {
              name = "image-root";
              paths = devPackages;
              pathsToLink = [ "/bin" "/etc" "/lib" "/share" ];
            };
            fakeRootCommands = ''
              mkdir -p ./home/user/.config ./workspaces ./tmp ./lib
              chmod 1777 ./tmp
              echo "user:x:1000:1000::/home/user:/bin/zsh" >> ./etc/passwd
              echo "user:x:1000:" >> ./etc/group
              chown -R 1000:1000 ./home/user
              chmod 755 ./home/user
              mkdir -p ./usr/bin
              ln -sf /bin/env ./usr/bin/env
              # Symlink chromium as google-chrome for compatibility
              ln -sf /bin/chromium ./bin/google-chrome
              for f in ${pkgs.glibc}/lib/ld-linux*.so*; do
                ln -sf "$f" ./lib/$(basename "$f")
              done
              # Fontconfig setup for Chromium
              mkdir -p ./etc/fonts
              ln -sf ${pkgs.fontconfig.out}/etc/fonts/fonts.conf ./etc/fonts/fonts.conf
              fc-cache -f 2>/dev/null || true
            '';
            config = {
              Env = [
                "LANG=C.UTF-8"
                "LANGUAGE=C.UTF-8"
                "LC_ALL=C.UTF-8"
                "NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
                "HOME=/home/user"
              ];
              User = "1000:1000";
              Cmd = [ "/bin/zsh" ];
            };
          };
        }
      );
    };
}
