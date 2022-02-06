# Distribution Packages

> These packages are not maintained by the Conduit maaintainers. They are third-party community contributions we have no control over.

## Debian

[Paul](https://wiki.debian.org/PaulVanTilburg) has done work on preparing Conduit for Debian packaging. See the [Debian directory](https://gitlab.com/famedly/conduit/-/tree/next/debian) for more info about this.

```bash
# You'll need cargo-deb to create a debian package:
cargo install cargo-deb
# Run this in the Conduit repo to compile and create a package:
cargo deb
```

## NixOS

[PimEyes](https://github.com/pimeys) has packaged
[Conduit for NixOS](https://search.nixos.org/packages?channel=unstable&show=matrix-conduit&from=0&size=50&sort=relevance&type=packages&query=matrix-conduit).

```bash
nix-env -iA nixos.matrix-conduit
```

## FreBSD Ports

Apparently, there is also a [FreeBSD Port of Conduit](https://www.freshports.org/net-im/conduit).

```bash
cd /usr/ports/net-im/conduit/ && make install clean
```
