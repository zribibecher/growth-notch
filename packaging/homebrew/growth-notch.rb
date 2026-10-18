# Template for the Homebrew tap (zribibecher/homebrew-tap, Casks/growth-notch.rb).
# After each release: update `version` and both sha256 values (`shasum -a 256 <dmg>`),
# and check the DMG file names against the release assets.
cask "growth-notch" do
  arch arm: "aarch64", intel: "x64"

  version "0.1.0"
  sha256 arm:   "REPLACE_WITH_SHA256_OF_AARCH64_DMG",
         intel: "REPLACE_WITH_SHA256_OF_X64_DMG"

  url "https://github.com/zribibecher/growth-notch/releases/download/v#{version}/GrowthNotch_#{version}_#{arch}.dmg"
  name "Growth Notch"
  desc "Ambient notch dashboard for YouTube creators"
  homepage "https://github.com/zribibecher/growth-notch"

  depends_on macos: ">= :monterey"

  app "GrowthNotch.app"

  zap trash: "~/Library/Application Support/com.techuntangled.growth-notch"
end
