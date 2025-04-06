{ pkgs, libnl-src, libiptc-src }: {
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

  libiptc = pkgs.stdenv.mkDerivation {
    name = "libiptc-static";

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

    src = libiptc-src;

    configurePhase = ''
      export CC=musl-clang
      aclocal
      autoreconf -vi
      ./configure \
          --enable-static \
          --disable-shared \
          --disable-ipv6 \
          --disable-nftables
    '';

    buildPhase = ''
      export CC=musl-clang
      make
    '';

    installPhase = ''
      mkdir -p $out/lib
      cp libiptc/.libs/*.a $out/lib
    '';
  };
}
