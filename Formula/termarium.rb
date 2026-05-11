class Termarium < Formula
  desc "Quiet terminal planetarium with a real bright-star sky map"
  homepage "https://github.com/7b7b7b/termarium"
  url "https://github.com/7b7b7b/termarium/archive/refs/tags/v0.1.6.tar.gz"
  sha256 "bf43d43704e53b07e02c27fc908b095db0ba1446779d3d190573c7436e8571d2"
  license all_of: ["MIT", "CC-BY-SA-4.0", "CC-BY-4.0"]
  head "https://github.com/7b7b7b/termarium.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/termarium --version")
    assert_match "Termarium star catalog", shell_output("#{bin}/termarium catalog-info")
  end
end
