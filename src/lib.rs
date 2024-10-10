use std::fs;

use zed::LanguageServerId;
use zed_extension_api as zed;

struct PklExtension {
    cached_jar_path: Option<String>,
}

impl PklExtension {
    fn language_server_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
    ) -> zed::Result<String> {
        if let Some(path) = &self.cached_jar_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "apple/pkl-lsp",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = format!("pkl-lsp-{version}.jar", version = release.version);

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        if !fs::metadata(&asset_name).map_or(false, |stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &asset_name,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&asset_name) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        Ok(self
            .cached_jar_path
            .insert(
                std::path::absolute(asset_name)
                    .map_err(|err| err.to_string())?
                    .to_str()
                    .ok_or("Path to pkl-lsp is not utf8")?
                    .to_owned(),
            )
            .clone())
    }
}

impl zed::Extension for PklExtension {
    fn new() -> Self {
        Self {
            cached_jar_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: worktree.which("java").ok_or("Java must be installed")?,
            args: vec![
                "-jar".into(),
                self.language_server_path(language_server_id)?,
            ],
            env: Default::default(),
        })
    }
}

zed::register_extension!(PklExtension);
