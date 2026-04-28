# Documentation: https://docs.brew.sh/Formula-Cookbook
class Ublx < Formula
  desc "TUI that turns a directory into a flat, navigable catalog with previews and metadata"
  homepage "https://github.com/thicclatka/ublx"
  url "https://github.com/thicclatka/UBLX/archive/refs/tags/v0.1.5.tar.gz"
  sha256 "c9a9abb4fb740cee731585b32eb3993ec0299d4cf1399cb3e1beaa419af30988"
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
