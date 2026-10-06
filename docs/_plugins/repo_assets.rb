# frozen_string_literal: true

# The site shows the app's icon and its screenshots. They live in the
# repository's assets/ folder, outside the site's source, so that there is one
# copy of each. This publishes the icon and every picture in
# assets/screenshots under /assets/images/.

module Tabletist
  class RepoAsset < Jekyll::StaticFile
    PUBLISHED_DIR = "/assets/images"

    def initialize(site, repo_root, dir, name)
      super(site, repo_root, dir, name)
      @relative_path = File.join(PUBLISHED_DIR, name)
    end

    def destination(dest)
      File.join(dest, PUBLISHED_DIR, name)
    end

    def url
      File.join(PUBLISHED_DIR, name)
    end
  end

  class RepoAssets < Jekyll::Generator
    safe true

    ICON = "assets/icon/tabletist.svg"
    SCREENSHOTS = "assets/screenshots/*.{png,webp}"

    def generate(site)
      repo_root = File.expand_path("..", site.source)
      screenshots = Dir.glob(SCREENSHOTS, base: repo_root).sort
      raise Jekyll::Errors::FatalException, "repo_assets: no screenshots in assets/screenshots" if screenshots.empty?

      [ICON, *screenshots].each do |path|
        full = File.join(repo_root, path)
        raise Jekyll::Errors::FatalException, "repo_assets: #{path} is missing" unless File.file?(full)

        site.static_files << RepoAsset.new(site, repo_root, File.dirname(path), File.basename(path))
      end
    end
  end
end
