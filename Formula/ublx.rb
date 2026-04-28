# Documentation: https://docs.brew.sh/Formula-Cookbook
class Ublx < Formula
  desc "TUI that turns a directory into a flat, navigable catalog with previews and metadata"
  homepage "https://github.com/thicclatka/ublx"
  url "https://github.com/thicclatka/ublx/archive/refs/tags/v0.1.4.tar.gz"
  sha256 "01c836f9f1ae1871c8dad930e3d4a92cf8e4a41c5d340c256918e25013ee607c"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on "pkgconf" => :build
  depends_on "rust" => :build

  depends_on "ffmpeg"
  depends_on "netcdf"
  depends_on "poppler"
  depends_on "tree"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Usage:", shell_output("#{bin}/ublx --help")
  end
end
