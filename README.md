# XLM - XIVLauncher Manager

> [!IMPORTANT]  
> While I personally use this tool daily without issue, you may run into problems that I have not. Please report any issues you experience on the issues tab.

An alternative method for launching XIVLauncher.Core on Linux, primarily built to avoid the pitfalls of using Flatpak XIVLauncher & Steam together. It allows for launching standalone or via a steam compatibility tool while providing nice features like launcher auto-updates (and steam overlay support!)

## Setup

Auto installers for the steam compatibility tool part of XLM are provided for the `Steam Deck`, `Flatpak`, `Snap` and `Native` versions of Steam. For any other use-case you will need to manually download the XLM binary from the [GitHub Releases Page](https://github.com/Blooym/xlm/releases/latest).

### Installers

Run one of the following commands to install XLM as a Steam compatibility tool. What command you need to run depends on how you have Steam installed. **These scripts CANNOT and SHOULD NOT be run with sudo or root permissions.**

**Steamdeck**:

```sh
sh -c "$(curl -fsSL https://raw.githubusercontent.com/Blooym/xlm/main/setup/install-steamdeck.sh)"
```

**Steam (Native)**
```sh
sh -c "$(curl -fsSL https://raw.githubusercontent.com/Blooym/xlm/main/setup/install-native.sh)"
```

**Steam (Snap)**
```sh
sh -c "$(curl -fsSL https://raw.githubusercontent.com/Blooym/xlm/main/setup/install-snap.sh)"
```

**Steam (Flatpak)**
```sh
sh -c "$(curl -fsSL https://raw.githubusercontent.com/Blooym/xlm/main/setup/install-flatpak.sh)"
```

After the installer has finished, please follow these steps to use the compatibility tool:
- Switch back to gaming mode (if on Steam Deck) or restart your Steam client otherwise.
- Navigate to your library and select "FINAL FANTASY XIV Online" or "FINAL FANTASY XIV Online Free Trial" if you are playing via the free trial or don't own the Steam edition of FFXIV. 
- Open the game properties menu and switch to the "compatibility" tab.
- Enable the "Force the use of a specific Steam Play compatibility tool" checkbox.
- From the box that appears select "XLCore [XLM]" (if this does not show, please make sure you properly restarted Steam).
- You can now launch the game as usual. XIVLauncher will be automatically installed and run for you.

### Passing extra arguments or environment variables on startup (Advanced & Optional)

When using the compatibility tool you have the option to pass extra launch arguments in two ways.

1. (For Users): You can add any available launch-command flag via Steam's "Launch Options" settings. You shouldn't need to do this by default, however it may be necessary if you would like to use a fork of XIVLauncher or for debugging and troubleshooting purposes.

2. (For Developers): You can set `--extra-launch-args` & `--extra-env-vars` during the `install-steam-tool` command. These values will be passed to the launch command every time XLM is ran and will ensure users use these additional arguments by default without additional steps. This will allow you to override key behaviours of XLM (such as permanently using a fallback secrets provider). This is also the only way to set extra environment variables.

More information on launch flags can be found by running `xlm launch --help` or [viewing the code (advanced)](https://github.com/Blooym/xlm/blob/229577915a049164bcc72ac4e604836702288b8b/src/commands/launch.rs#L28).

#### Using a fork of XIVLauncher

To use a fork of XIVLauncher you can add the flags `--xlcore-repo-owner` and `--xlcore-repo-name` to the Steam "Launch Arguments" section. 

Forks of XIVLauncher can also offer their own install scripts for XLM that automate this process for you so you don't have to manually tinker, so do check to see if one exists for the fork you want to use!

### "No secrets provider installed or configured"

This means that XIVLauncher was unable to find a secure way to store your passwords. This is usually because you don't have a secrets manager like GNOME Keyring or KDE Wallet installed on your system. It's recommended you install a recognised and well known secrets manager to solve this problem.

If you still run into this issue even with a secrets manager installed on your system, use the fallback file storage provider offered by XIVLauncher; You can tell XLM to ask XIVLauncher to enable this by adding `--use-fallback-secrets-provider` to Steam's "Launch Arguments" section. Please note that this has been done for you if you used the Steam Deck or Flatpak installation scripts. 
