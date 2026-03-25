cask "lattice" do
  version "0.1.0"
  sha256 "PLACEHOLDER"

  url "https://github.com/puneet2019/lattice/releases/download/v#{version}/Lattice_#{version}_aarch64.dmg"

  name "Lattice"
  desc "AI-Native Spreadsheet for macOS with built-in MCP server"
  homepage "https://github.com/puneet2019/lattice"

  depends_on macos: ">= :ventura"

  app "Lattice.app"

  zap trash: [
    "~/Library/Application Support/Lattice",
    "~/Library/Preferences/com.lattice-app.plist",
  ]
end
