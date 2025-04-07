{ pkgs, libnl-src }: {
  libnl = pkgs.stdenv.mkDerivation {
    name = "libnl-static";

    nativeBuildInputs = with pkgs; [
      musl
      clang
      autoconf
      automake
      libtool
      pkg-config
      autoconf-archive
      flex
      bison
    ];

    src = libnl-src;

    configurePhase = ''
      export CC=musl-clang
      export CXX=musl-clang
      aclocal
      autoreconf -vi
      ./configure \
          --enable-static \
          --disable-shared
    '';

    buildPhase = ''
      export CC=musl-clang
      export CXX=musl-clang
      make
    '';

    installPhase = ''
      mkdir -p $out/lib
      cp lib/.libs/*.a $out/lib
    '';
  };
}
