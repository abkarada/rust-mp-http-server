# Maintainer: Abdurrahman Karadağ <abdurrahman.karadag@roftcore.com>
pkgname=rust-mp-http-server
pkgver=0.1.0
pkgrel=1
pkgdesc="High-Performance Multi-Protocol HTTP/1.1, HTTP/2, and HTTP/3 QUIC Server"
arch=('x86_64' 'aarch64')
url="https://github.com/abkarada/rust-mp-http-server"
license=('MIT')
depends=('gcc-libs')
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "$pkgname-$pkgver"
  cargo build --release --locked
}

package() {
  cd "$pkgname-$pkgver"
  install -Dm755 "target/release/high-performance-http-server" "$pkgdir/usr/bin/mp-http-server"
  install -Dm644 "mp-http-server.service" "$pkgdir/usr/lib/systemd/system/mp-http-server.service"
}
