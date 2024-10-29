use anyhow::{bail, Result};
use bytes::Buf;
use clap::Parser;
use eframe::egui::{self, Layout};
use flate2::read::GzDecoder;
use log::{debug, error, info};
use reqwest::Url;
use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{ErrorKind, Write},
    path::PathBuf,
    sync::RwLock,
};
use tar::Archive;
use tokio::process::Command;
use winit::platform::wayland::EventLoopBuilderExtWayland;

const XLCORE_VERSIONDATA_FILENAME: &str = "versiondata";
const XIVLAUNCHER_BIN_NAME: &str = "XIVLauncher.Core";

/// Whether all egui windows should close when they next redraw.
static UI_SHOULD_CLOSE: RwLock<bool> = RwLock::new(false);

/// Install or update XIVLauncher and then open it.
#[derive(Debug, Clone, Parser)]
pub struct LaunchCommand {
    /// The name of the GitHub repository owner for XIVLauncher.
    #[clap(default_value = "goatcorp", long = "xlcore-repo-owner")]
    xlcore_repo_owner: String,

    /// The name of the GitHub repository for XIVLauncher.
    #[clap(default_value = "XIVLauncher.Core", long = "xlcore-repo-name")]
    xlcore_repo_name: String,

    /// The name of the release tar.gz archive that contains a self-contained XIVLauncher.
    #[clap(
        default_value = "XIVLauncher.Core.tar.gz",
        long = "xlcore-release-asset"
    )]
    xlcore_release_asset: String,

    /// The URL to a release of XIVLauncher.Core. This conflicts with `xlcore-repo-owner` and `xlcore-repo-name`
    /// as it overrides the default git-based release system.
    ///
    /// This should be a URL prefix that contains:
    /// - A file called `version` that contains the version of the release.
    /// - A file with the name of `<xlcore-release-asset>` that contains the tar.gz archive of the release.
    #[clap(
        long = "xlcore-web-release-url-base",
        conflicts_with = "xlcore_repo_name",
        conflicts_with = "xlcore_repo_owner"
    )]
    xlcore_web_release_url_base: Option<Url>,

    /// The location of a tarball that contains a static build of aria2c.
    #[clap(
        default_value = "https://github.com/rankynbass/aria2-static-build/releases/download/v1.37.0-2/aria2-static.tar.gz",
        long = "aria-download-url"
    )]
    aria_download_url: Url,

    /// The location where the XIVLauncher should be installed.
    #[clap(default_value = dirs::data_local_dir().unwrap().join("xlcore").into_os_string(), long = "install-directory")]
    install_directory: PathBuf,

    /// Use a fallback secrets provider with XIVLauncher instead of the system provided.
    /// Used when no system secrets provider is available and credentials should still be saved.
    #[clap(default_value_t = false, long = "use-fallback-secret-provider")]
    use_fallback_secret_provider: bool,

    /// Skip checking for XIVLauncher updates. This will not prevent XIVLauncher from installing if it isn't installed.
    #[clap(default_value_t = false, long = "skip-update")]
    skip_update: bool,
}

impl LaunchCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        debug!("Attempting launch with args: {self:?}");

        // Query the GitHub API or web release Url for release information.
        let (remote_version, remote_release_url) = match self.xlcore_web_release_url_base {
            Some(url) => Self::get_release_web(url, &self.xlcore_release_asset).await?,
            None => {
                Self::get_release_github(
                    &self.xlcore_repo_owner,
                    &self.xlcore_repo_name,
                    &self.xlcore_release_asset,
                )
                .await?
            }
        };

        // Install XIVLauncher or do an update check if version data already exists locally.
        match fs::read_to_string(self.install_directory.join(XLCORE_VERSIONDATA_FILENAME)) {
            Ok(ver) => {
                if !self.skip_update {
                    if ver == remote_version {
                        info!("XIVLauncher is up to date!");
                    } else {
                        Self::open_xlm_wait_ui();
                        info!("XIVLauncher is out of date - starting update");
                        Self::install_or_update_xlcore(
                            &remote_version,
                            remote_release_url,
                            self.aria_download_url,
                            &self.install_directory,
                        )
                        .await?;
                        info!("Successfully updated XIVLauncher to the latest version.")
                    }
                } else {
                    info!("Skip update enabled, not attempting to update XIVLauncher.")
                }
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    Self::open_xlm_wait_ui();
                    info!("Unable to obtain local version data for XIVLauncher - installing latest release");
                    Self::install_or_update_xlcore(
                        &remote_version,
                        remote_release_url,
                        self.aria_download_url,
                        &self.install_directory,
                    )
                    .await?;
                    info!("Successfully installed XIVLauncher")
                } else {
                    error!(
                        "Something went wrong whilst checking for XIVLauncher: {:?}",
                        err
                    );
                }
            }
        };

        Self::close_xlm_wait_ui();

        info!("Starting XIVLauncher");

        let mut cmd = Command::new(self.install_directory.join(XIVLAUNCHER_BIN_NAME));
        if self.use_fallback_secret_provider {
            cmd.env("XL_SECRET_PROVIDER", "FILE");
        }
        let cmd = cmd
            .env("XL_PRELOAD", env::var("LD_PRELOAD").unwrap_or_default()) // Write XL_PRELOAD so it can maybe be passed to the game later.
            .env("XL_SCT", "1") // Needed to trigger compatibility tool mode in XIVLauncher. Otherwise XL_PRELOAD will be ignored.
            .env_remove("LD_PRELOAD") // Completely remove LD_PRELOAD otherwise steam overlay will break the launcher text.
            .spawn()?
            .wait()
            .await?;

        info!("XIVLauncher process exited with exit code {:?}", cmd.code());

        Ok(())
    }

    async fn get_release_github(
        xlcore_repo_owner: &String,
        xlcore_repo_name: &String,
        xlcore_release_asset: &String,
    ) -> Result<(String, Url)> {
        let octocrab = octocrab::instance();
        let repo = octocrab.repos(xlcore_repo_owner, xlcore_repo_name);
        let release = match repo.releases().get_latest().await {
            Ok(release) => release,
            Err(err) => {
                bail!(
                    "Failed to obtain release information for {}/{}: {:?}",
                    xlcore_repo_owner,
                    xlcore_repo_name,
                    err.source()
                );
            }
        };

        let release_url = release
            .assets
            .iter()
            .find(|asset| &asset.name == xlcore_release_asset);

        if let Some(asset) = release_url {
            Ok((release.tag_name, asset.browser_download_url.clone()))
        } else {
            bail!(
                "Failed to find asset {} in release {}",
                xlcore_release_asset,
                release.tag_name
            );
        }
    }

    async fn get_release_web(base_url: Url, xlcore_release_asset: &str) -> Result<(String, Url)> {
        let version_url = base_url.join("version")?;
        let release_url = base_url.join(xlcore_release_asset)?;

        info!("XLCore web release asset url:{}", release_url);
        info!("XLCore web release version url: {}", version_url);

        let response = reqwest::get(version_url).await?;
        if !response.status().is_success() {
            bail!("{}", format!("{:?}", response.error_for_status()))
        }
        Ok((response.text().await?, release_url))
    }

    /// Creates a new XLCore installation or overwrites an existing XLCore installion with a new one.
    async fn install_or_update_xlcore(
        release_version: &String,
        release_url: Url,
        aria_download_url: Url,
        install_location: &PathBuf,
    ) -> anyhow::Result<()> {
        // Download and decompress XLCore.
        {
            info!("Downloading release from {}", release_url,);
            let response = reqwest::get(release_url).await?;
            let bytes = response.bytes().await?;
            let mut archive = Archive::new(GzDecoder::new(bytes.reader()));
            let _ = fs::remove_dir_all(install_location);
            fs::create_dir_all(install_location)?;
            info!("Unpacking release tarball");
            archive.unpack(install_location)?;
            info!("Wrote XIVLauncher files");
        }

        {
            // Download and write aria2c
            let response = reqwest::get(aria_download_url).await?;
            let bytes = response.bytes().await?;
            let mut archive = Archive::new(GzDecoder::new(bytes.reader()));
            info!("Unpacking aria2c tarball");
            archive.unpack(install_location)?;
            info!("Wrote aria2c binary");
        }

        {
            // Write version info into the release.
            let mut file = File::options()
                .write(true)
                .create(true)
                .truncate(true)
                .append(false)
                .open(install_location.join(XLCORE_VERSIONDATA_FILENAME))?;
            file.write_all(release_version.as_bytes())?;
            info!("Wrote versiondata with {}", release_version);
        }

        Ok(())
    }

    /// Starts a new Tokio task and displays an egui window displaying a "XIVLauncher is starting" message.
    /// The egui window blocks inside of the task meaning it cannot be killed by aborting the thread.
    /// To close the window you can call [`Self::close_xlm_wait_ui`] which will close all existing windows.
    fn open_xlm_wait_ui() {
        *UI_SHOULD_CLOSE.write().unwrap() = false;
        tokio::task::spawn(async move {
            eframe::run_simple_native(
                "XLM",
                eframe::NativeOptions {
                    event_loop_builder: Some(Box::new(|event_loop_builder| {
                        event_loop_builder.with_any_thread(true);
                    })),
                    viewport: egui::ViewportBuilder::default()
                        .with_inner_size([800.0, 500.0])
                        .with_resizable(false)
                        .with_decorations(false),
                    ..Default::default()
                },
                move |ctx, _frame| {
                    if *UI_SHOULD_CLOSE.read().unwrap() {
                        std::process::exit(0);
                    }

                    ctx.set_pixels_per_point(1.5);
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.with_layout(
                            Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                ui.heading("Preparing XIVLauncher, this may take a moment...");
                            },
                        );
                    });
                },
            )
            .unwrap();
        });
    }

    /// Closes any running egui windows regardless of the thread they're running on.
    fn close_xlm_wait_ui() {
        *UI_SHOULD_CLOSE.write().unwrap() = true;
    }
}
