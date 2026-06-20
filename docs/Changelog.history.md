## v2.5.1

### 🐞 Bug Fixes

- Backup settings function anomaly
- Fixed Windows node interaction anomaly

---

## v2.5.0

> [!IMPORTANT]
> Version Note: Clash Verge versioning follows x.y.z: x for major architectural changes, y for new features, z for bug fixes.

- **Mihomo(Meta) Kernel upgraded to v1.19.25**

### 🐞 Bug Fixes

- Fixed system proxy not completely shut down in PAC mode after turning off
- Fixed macOS potential freeze when toggling proxy
- Fixed scheduled update timer not refreshing immediately after modification
- Fixed Linux TUN disable not taking effect immediately
- Fixed system proxy close sequence logic (preventing unsaved close state during quick exit)
- Fixed Linux shortcut mapping error
- Fixed Linux Wayland compatibility rendering error
- Fixed Linux edge case failing to load pages

### ✨ Features

- Subscription QR code sharing
- Added macOS tray speed display
- Shortcut operations notify results
- App auto update (background download, auto install on next launch)

### 🚀 Optimizations

- Optimized macOS system proxy reading performance
- Optimized frontend CPU performance
- Robust service mode and edge case kernel recovery
- Optimized subscription TLS update compatibility under whitelist networks

### 👙 UI & Style

- Sticky Scroll effect for proxy groups

---

## v2.4.7

### 🐞 Bug Fixes

- Fixed Windows switch TUN mode anomaly when running as administrator
- Fixed silent start conflict with auto lightweight mode
- Fixed inability to return to main interface after entering lightweight mode
- Occasional failure when switching profiles
- Fixed regression of extreme delay when switching nodes or modes
- Fixed website latency tests passing through proxy when proxy is off
- Fixed inaccurate Gemini unlock tests

### ✨ Features

- (None)

### 🚀 Optimizations

- Optimized subscription error notification, only on manual trigger
- Hide subscription info in logs
- Optimized UI copy and text
- Optimized latency when switching nodes
- Optimized exit hotkey display in tray
- Optimized node info refresh on first launch
- Linux uses built-in window controls by default
- Implemented exclusion check for custom subnets
- Removed redundant auto-backup trigger conditions
- Restored mihomo config syntax hints in built-in editor
- Website latency tests use real TLS handshake delay
- System proxy indicator (icon) uses real proxy state
- System proxy switch indicator verifies if it points to Verge
- System proxy switch modified to optimistic update mode to improve user experience

---

## v2.4.6

> [!IMPORTANT]
> After multiple rounds of tuning and corrections, this is our most satisfactory milestone release since 2.0. All users are highly recommended to upgrade immediately.

### 🐞 Bug Fixes

- Fixed slow proxy info refresh on first launch
- Fixed infinite IP geolookup queries when offline
- Fixed WebDAV page retry logic
- Fixed Linux service mode permissions installed via GUI not as expected
- Fixed macOS unable to set proxy correctly due to network interface order
- Fixed unable to operate tray after waking up from sleep
- Fixed inconsistent icon semantics for current node on homepage
- Fixed configuration not reloaded in time when importing subscription via URL scheme
- Fixed line number display logic in rules page
- Fixed Windows tray failing to open logs
- Fixed KDE first startup error

### ✨ Features

- Upgraded Mihomo kernel to latest
- Supported auto-latency test interval in subscription settings
- Added connection tunnel management UI, supporting visual add/delete tunnel config
- GUI support for Masque protocol

### 🚀 Optimizations

- Report more detailed errors when service installation fails
- Prevent invalid subscription URLs from blocking scheme import
- Use 114.114.114.114 when macOS TUN overrides DNS
- Connectivity test replaced with faster http://1.0.0.1
- Added clear search buttons to filters in connections, rules, logs, etc.
- Chain proxy displays clear ingress, egress, and data flow indicators
- Optimized IP info card
- Beautified proxy group icon styles
- Removed redundant service binaries in Linux resources folder

---

## v2.4.5

- **Mihomo(Meta) Kernel upgraded to v1.19.19**

### 🐞 Bug Fixes

- Fixed macOS wired network DNS hijack failure
- Fixed Monaco editor right-click menu display anomaly
- Fixed port occupation check when setting proxy port
- Fixed Monaco editor initialization stuck on Loading
- Fixed fields in `config.yaml` / `profiles.yaml` not correctly restored during backup restoration
- Fixed Windows system theme sync issue
- Fixed URL Schemes failing to import properly
- Fixed unable to install TUN service under Linux
- Fixed potential port occupation false alarms
- Fixed setting allowed external control sources not taking effect immediately
- Fixed frontend performance regression

### ✨ Features

- Supported advanced filtering and searching on proxy page
- Added import backup button in backup settings page
- Allowed changing notification popup position
- Supported collapsing navigation bar (navigation bar right-click menu / UI settings)
- Allowed displaying outbound mode in tray primary menu
- Allowed disabling proxy group display in tray
- Supported importing AnyTLS URI config directly in "Edit Nodes"
- Supported disabling "validate proxy bypass formats"
- Added visual editor for system proxy bypass and TUN excluded custom subnets

### 🚀 Optimizations

- In-app update log supports parsing and rendering HTML tags
- Performance optimized frontend and backend resources when rendering traffic charts
- Attempted to disable WebKit DMABUF rendering under Linux NVIDIA environments to avoid potential issues
- Changed Windows startup to scheduled task implementation
- Improved tray and window operation rate limiting implementation
- When adding a node using "Edit Nodes", automatically add the node to the first position of the first `select` type proxy group
- Hidden scrollbars of sidebar navigation and floating jump navigation
- Completed GUI support for AnyTLS / Mieru / Sudoku
- Further restricted service IPC permissions on macOS and Linux
- Removed redundant 3-second delay in Windows autostart scheduled task
- Right-click error notification can copy error details
- Optimized execution flow when saving TUN settings to avoid UI freeze
- Added `deb` / `rpm` dependency `libayatana-appindicator`
- Expanded connection table title sort click area to full column width
- Display loading overlay during backup restoration, no need to manually close dialogs anymore

---

## v2.4.4

- **Mihomo(Meta) Kernel upgraded to v1.19.17**

### 🐞 Bug Fixes

- Linux unable to switch TUN stack
- macOS service startup item display name (experimental change)
- macOS unexpected Tproxy port settings
- Traffic chart scaling anomaly
- PAC auto proxy script content cannot be adjusted dynamically
- Compatible with upgrading from old version service mode
- Monaco editor line count limit
- Deleted nodes in manual groups causing config load failure
- Dashboard and tray status out of sync
- Completely fixed macOS connection page display anomaly
- Windows client failed to listen to shutdown signals
- Fixed proxy button and highlight status out of sync
- Fixed sidebar potential failure to jump correctly
- Fixed incorrect icon encoding in unlock tests for some regions
- Changed IP detection page refresh to only update when necessary
- Fixed crash when inputting incomplete regex in search box
- Fixed brief flicker when creating window in non-Simplified Chinese environments or dark themes
- Fixed progress bar anomaly during updates
- Fixed kernel unavailable issue caused by kernel upgrade failure
- Fixed macOS install/uninstall service prompts not matching actions
- Fixed menu sorting mode drag-and-drop anomaly
- Fixed abnormal check status before proxy groups in tray menu
- Fixed Windows custom title bar buttons hover status residue after minimize / close
- Fixed unable to expand proxy groups when overwriting `config.yaml` directly
- Fixed macOS system tray icon color flickering at app startup
- Fixed global hotkeys hijacking other app keys under silent start mode
- Fixed homepage current node card sorting by delay displaying `timeout` nodes before normal nodes when list is opened

### ✨ Features

- Supported sorting for each column on connection page
- Implemented optional auto-backup
- Connection page supports viewing closed connections (up to 500 recently closed connections)
- Log page supports reverse chronological order
- Added global hotkey for "Re-activate Subscription"
- WebView2 Runtime fix build upgraded to 133.0.3065.92
- Sidebar right-click menu added "Restore Default Order"
- Added TUN "auto-redirect" configuration support under Linux, disabled by default

### 🚀 Optimizations

- Network requests changed to use rustls, improving TLS compatibility
- rustls avoids subscription import failure due to server certificate chain config issues or newer TLS requirements
- Replaced frontend info editing components to provide better performance
- Optimized backend memory and performance
- Prevented potential TUN disable failure during exit
- Brand new i18n support method
- Optimized backup settings layout
- Optimized traffic chart performance, implementing dynamic FPS and window blur auto-pause
- Performance optimized system status fetching
- Optimized tray menu current subscription detection logic
- Optimized connection page table rendering
- Optimized chain proxy UI feedback
- Optimized app restart resource cleanup logic
- Optimized frontend data refresh
- Optimized traffic sampling and data processing
- Optimized resource cleanup performance during app restart/exit, significantly shortening execution time
- Optimized frontend WebSocket connection mechanism
- Improved old version Service re-installation detection flow
- Optimized macOS, Linux, and Windows system signal handling
- Chain proxy only displays Selector type rule groups
- Optimized Windows system proxy settings, no longer relying on `sysproxy.exe` to set proxy

---

## v2.4.3

**Release Codename: Lan**

Codename meaning: Lan symbolizes smoothness and integration. This release focuses on stability, compatibility, performance, and experience optimization, comprehensively improving overall reliability.

Special thanks to @Slinetrac, @oomeow, @Lythrilla, @Dragon1573 for their excellent contributions.

### 🐞 Bug Fixes

- Optimized service mode reinstall logic, avoiding unnecessary checks
- Fixed lightweight mode exit unresponsiveness
- Fixed tray lightweight mode exit/enter support
- Fixed tray status refresh not relying on window creation flow when starting silently and entering lightweight mode automatically
- Fixed inconsistent icon sizes under macOS Tun/System Proxy mode
- Tray node switching no longer displays hidden groups
- Fixed frontend IP check unable to use ipapi, ipsb providers
- Fixed macOS system proxy unable to open after Tun mode is enabled
- Fixed page freeze during configuration editing, generation, or kernel restart under service mode
- Fixed Webdav backup restoration not restarting
- Fixed Linux startup requiring manual proxy settings to work normally
- Fixed subscription page not updating when adding or importing subscription files
- Fixed system proxy guard not working
- Fixed UI rendering anomalies on multi-monitor displays under KDE + Wayland
- Fixed Windows title bar color anomaly at first launch under dark mode
- Fixed silent start not loading full WebView
- Fixed Linux WebKit network process crashes
- Fixed subscription import failures
- Fixed successful imports showing failure notifications
- Fixed app freeze when service is unavailable and Tun mode auto-closes
- Fixed failure to delete actual files when deleting subscriptions
- Fixed macOS connection page display anomaly
- Fixed rule config shared globally across profiles causing switch reset issues
- Fixed UI rendering issues with some GPUs under Linux Wayland
- Fixed auto-update causing version rollbacks
- Fixed homepage custom cards failing when switching lightweight mode
- Fixed floating navigation failure
- Fixed keypad hotkey mapping error
- Fixed frontend failing to refresh operation status in time
- Fixed macOS lightweight mode status out of sync when exiting from Dock
- Fixed Linux system theme switch not taking effect
- Fixed "allow auto update" field causing manual subscription refresh to fail
- Fixed lightweight mode tray status out of sync
- Fixed app crash when importing subscriptions with one-click import

### ✨ Features

- **Mihomo(Meta) Kernel upgraded to v1.19.15**
- Supported frontend modification of logs (max file size, max retention count)
- Added graphical setup for chain proxy
- Added toggle between system title bar and app title bar (Settings - UI Settings - Prefer System Title Bar)
- Listen to shutdown events, automatically closing system proxy
- Homepage "Current Node" card added "Latency Test" button
- Added batch profile selection feature
- Windows / Linux / macOS listen to shutdown signals, gracefully restoring network settings
- Added local backup feature
- Homepage "Current Node" card added auto-latency test switch (disabled by default)
- Allowed independent control of subscription auto-updates
- Tray "More" added "Close All Connections" button
- Added left menu bar sorting feature (right-click on left menu bar)
- Tray "Open Directory" added "App Logs" and "Kernel Logs"

### 🚀 Optimizations

- Refactored and simplified service mode startup detection flow, eliminating duplicate checks
- Refactored and simplified window creation flow
- Refactored log system, single log defaults to max 10 MB
- Optimized frontend resource usage
- Improved macOS system proxy configuration method
- Optimized TUN mode availability determination
- Removed system-level notifications for streaming media checks (using in-app notifications instead)
- Optimized backend i18n resource usage
- Improved Linux tray support and added `--no-tray` option
- Linux now defaults to restoring TUN stack to mixed mode in newly generated configs
- Added safety protection and secure fallback URLs for latency test settings
- Updated Wayland compositor detection logic to retain native Wayland backend in Hyprland sessions
- Improved Windows and Unix service connection methods and permissions to prevent connection failures
- Changed default kernel log level to Info
- Supported re-opening application via desktop shortcuts
- Supported pressing Enter in subscription input to import
- Automatically refresh node order when selecting sorting by latency
- Automatically restart core when configuration reload fails
- Wait for service readiness before enabling TUN
- Close TUN before uninstalling it
- Optimized app startup page
- Optimized homepage current node support for MATCH rules
- Allowed modifying "Floating Navigation Delay" in UI Settings
- Added error prompts for hotkey binding failures
- Included Mihomo-go122 by default on macOS 10.15 and higher to resolve Intel Mac kernel compatibility issues
- Disabled system tray TUN mode menu when TUN mode is unavailable
- Improved subscription update mechanism; if it still fails, turn on "allow dangerous certificates" in subscription settings
- Allowed setting Mihomo port range to 1000(inclusive) - 65536(inclusive)

---

## v2.4.2

### ✨ Features

- Added tray node selection

### 🚀 Optimizations

- Optimized frontend homepage loading speed
- Optimized frontend unused i18n file caching
- Optimized backend memory usage
- Optimized backend startup speed

### 🐞 Bug Fixes

- Fixed homepage node switching failure
- Fixed and optimized service check flow
- Fixed redirect error when importing subscription addresses introduced in 2.4.1
- Fixed rpm/deb package naming issues
- Fixed tray lightweight mode status detection anomaly
- Fixed crash when importing subscription via scheme
- Fixed singleton detection failure
- Fixed inability to connect to kernel during startup phase
- Fixed subscription import failing Auth Basic

### 👙 UI & Style

- Simplified and improved proxy settings styles

---

## v2.4.1

### 🏆 Major Improvements

- **App Response Speedup**: Adopted a brand new asynchronous processing architecture, significantly improving app responsiveness and stability.

### ✨ Features

- **Mihomo(Meta) Kernel upgraded to v1.19.13**

### 🚀 Optimizations

- Optimized hotkey response speed, improving shortcut experience
- Improved service management responsiveness, reducing wait times
- Boosted file and configuration processing performance
- Optimized task management and log recording efficiency
- Optimized async memory management, reducing memory usage and improving multi-tasking efficiency
- Optimized startup phase initialization performance

### 🐞 Bug Fixes

- Fixed potential response delay in some app operations
- Fixed potential concurrency issues in task management
- Fixed app restart via tray failing to recover
- Fixed subscriptions failing to import under certain conditions
- Fixed unable to create new subscription when using remote links
- Fixed TUN switch status issue after uninstalling service
- Fixed crash when switching subscriptions rapidly on the page
- Fixed unable to restore environment when working directory is lost
- Fixed crash when restoring from lightweight mode

### 👙 UI & Style

- Unified proxy settings styles

### 🗑️ Removed

- Removed automatic cleanup of expired subscriptions during startup phase

---

## v2.4.0

**Release Codename: Rong**

Codename meaning: "Rong" symbolizes integration and permeability, implying that the new version closely connects all parts of the system through a new IPC communication mechanism, breaking barriers to achieve more efficient data flow and comprehensive performance optimization.

### 🏆 Major Improvements

- **Core Communication Architecture Upgrade**: Adopted a new communication mechanism, improving performance and stability.
- **Traffic Monitoring System Refactored**: Brand new traffic monitoring interface, supporting richer data displays.
- **Data Caching Optimized**: Improved configuration and node data caching, boosting response speed.

### ✨ Features

- **Mihomo(Meta) Kernel upgraded to v1.19.12**
- Added copy version info button
- Enhanced traffic monitoring, supporting more detailed data analysis
- Added multiple display modes for traffic charts
- Added force refresh configuration and node cache feature
- Homepage traffic stats support viewing scale line details

### 🚀 Optimizations

- Comprehensively improved data transmission and processing efficiency
- Optimized memory usage, reducing system resource consumption
- Improved traffic chart rendering performance
- Optimized configuration and node refresh strategy, extending it from 5 seconds to 60 seconds
- Improved data caching mechanism, reducing duplicate requests
- Optimized async program performance

### 🐞 Bug Fixes

- Fixed system proxy status detection and display inconsistency
- Fixed system theme window color inconsistency
- Fixed special characters URL processing issues
- Fixed cache out-of-sync after configuration changes
- Fixed Windows installer autostart configuration issue
- Fixed macOS Dock icon restoring window issue
- Fixed Linux KDE/Plasma abnormal title bar buttons
- Fixed node speed test feature anomaly after architecture upgrade
- Fixed traffic stats feature anomaly after architecture upgrade
- Fixed log feature anomaly after architecture upgrade
- Fixed external controller CORS configuration saving issue
- Fixed homepage port display inconsistency
- Fixed homepage traffic stats scale line display issue
- Fixed log page button functions confusion
- Fixed log level setting saving issue
- Fixed log level abnormal filtering
- Fixed clean log days feature anomaly
- Fixed occasional startup freeze issues
- Fixed virtual NIC switch status under management mode on homepage

### 🔧 Technical Improvements

- Unified using the new kernel communication method
- Added external controller configuration interface
- Improved cross-platform compatibility support

---

## v2.3.2

### 🐞 Bug Fixes

- Fixed system proxy port out-of-sync issue
- Fixed custom `css` background image failing to take effect
- Fixed race condition hang when clicking tray icon rapidly in lightweight mode
- Fixed auto lightweight mode failing when both silent start and auto lightweight mode are enabled
- Fixed tray toolbar lightweight mode status synchronization under silent start
- Fixed importing non-http scheme links
- Fixed page loading hang and out-of-sync cache when switching nodes
- Fixed unable to delete shortcut/registry after renaming to `Clash Verge`
- Fixed DNS override `fallback` `proxy server` `nameserver` `direct Nameserver` fields allowing empty values
- Fixed DNS override `nameserver-policy` failing to identify `geo` database
- Fixed search box special characters crash
- Fixed Windows startup name mismatch with exe name
- Fixed Mihomo core log level display should be greater than settings level

### ✨ Features

- Clean leftover core processes in `sidecar` mode to prevent run errors
- New tray icons for macOS TUN and system proxy mode (experimental)
- Shortcut events notify via system notifications
- Added external `cors` control panel

### 🚀 Optimizations

- Refactored profile switch logic, allowing cancellation at any time to prevent hangs
- Introduced event-driven proxy manager, optimizing proxy config updates to prevent hangs
- Improved subscription card traffic usage ratio calculation accuracy
- Optimized backend cache refresh mechanism, supporting millisecond TTL (default 3000ms) to reduce duplicate requests and improve performance; force refresh backend data when switching nodes for real-time UI updates
- Decoupled frontend data fetching and backend cache refresh to improve node switching speed and consistency

### 🗑️ Removed

- Removed macOS tray network rate display

### 🌐 Internationalization

- Fixed missing and inconsistent translations

---

## v2.3.1

### 🐞 Bug Fixes

- Added config validation to fix "No such file or directory (os error 2)" error when upgrading from older versions
- Fixed extension script escape errors
- Fixed macOS Intel x86 build error causing runtime failure
- Fixed white border issue on Linux
- Fixed tray unresponsiveness
- Fixed tray failing to exit lightweight mode and restore window
- Fixed potential hang when switching subscriptions rapidly

### ✨ Features

- Added window-state management and recovery

### 🚀 Optimizations

- Optimized unified tray response
- Optimized silent start + autostart lightweight mode operation
- Reduced frontend potential memory leaks to improve runtime performance
- Optimized React state, side-effects, data fetching, and cleanup flows

---

## v2.3.0

**Release Codename: Yu**

Codename meaning: 'Yu' symbolizes control and guardianship, representing the comprehensive mastery and enhancement of system stability, safety, and user experience in this version.

Although the `external-controller` secret is now automatically completed with a default value and is not allowed to be empty, **it is still recommended to modify the secret manually to improve security**.

### ⚠️ Known Issues

- Only briefly tested under the GNOME desktop environment on Ubuntu 22.04/24.04 and Fedora 41. Compatibility with other Linux distributions is not guaranteed.
- macOS:
  - After successful auto-upgrade on macOS, please close the program and wait 30 seconds before restarting. Due to macOS port release characteristics, you need to wait 30 seconds after uninstalling the service before restarting the app to restore core communications.
  - Background wallpaper is mainly light, causing dark Tray icons to flicker;
  - Color Tray icon colors are relatively light;
- Window state manager has upstream issues, temporarily removed window size/position memory.

### 🐞 Bug Fixes

- Fixed page hang when switching proxy modes rapidly on homepage
- Fixed macOS shortcut close window failing to trigger auto lightweight mode
- Fixed silent start abnormal window creation/close flow
- Fixed Windows incorrectly registered global hotkey `Ctrl+Q`
- Fixed unlock test error messages and network type errors during VLESS URL decoding
- Fixed system proxy status anomaly after switching custom proxy address
- Fixed macOS TUN default invalid network interface name
- Fixed tray UI out of sync after changing subscription
- Fixed unable to enable TUN mode immediately after installing service mode
- Fixed unable to delete `.window-state.json`
- Fixed unable to modify HTTP request timeout for profile update
- Fixed `getDelayFix` hook exception
- Fixed homepage failing to display proxy groups when external extension script overrides them
- Fixed Verge export diagnostic version out of sync with settings page
- Fixed settings page potential load failure when switching language
- Fixed hyphen handling in editor
- Fixed privilege escalation vulnerability by switching to authenticated IPC
- Fixed silent start unable to use auto lightweight mode
- Fixed JS script escaping special characters error
- Fixed macOS silent start incorrectly launching Dock icon

### ✨ Features

- **Mihomo(Meta) Kernel upgraded to v1.19.10**
- Supported setting proxy address to non-`127.0.0.1` to improve WSL compatibility
- System Proxy Guard: detects unexpected changes and auto-restores
- Tray displays current lightweight mode status
- Disconnect established connections when system proxy is turned off
- WebDAV features:
  - Added UA request headers
  - Supported directory redirection
  - Backup directory check and upload retry mechanism
- Auto subscription update mechanism:
  - Added request timeout to prevent hangs
  - Supported retrying subscription update under proxy status
  - Supported clicking subscription card to switch next auto-update time, with update status tooltip
- DNS settings support Hosts configuration
- Homepage proxy nodes support sorting
- Supported service mode manual uninstall, falling back to Sidecar mode
- Core state management supports switching, upgrading, restarting
- Config loading phase autocompletes `external-controller secret`
- Added log auto-cleanup interval option (including 1 day)
- Added Zashboard jump link
- Use system default window manager

### 🚀 Optimizations

- **System related:**
  - Optimized system proxy Bypass settings
  - Optimized proxy settings update logic and guard mechanism
  - Adjusted Windows startup method to Startup folder to solve self-start issue in admin mode
- **Performance & Stability:**
  - Asynchronized configuration loading, UI startup, event notification to resolve lag
  - Optimized MihomoManager and window creation
  - Changed kernel log level to `warn` to reduce noise
  - Refactored main process and notification system to improve responsiveness and decoupling
  - Optimized network request and error handling
  - Added network manager to prevent resource competition causing UI hangs
  - Optimized profile loading memory usage
  - Optimized memory usage for caching Mihomo proxy and providers info
- **Frontend & Interface:**
  - Rules page auto-refreshes data when switching
  - Non-active subscription edits no longer trigger config reload
  - Optimized tray rate display, disabled by default on macOS
  - Windows shortcut name renamed to `Clash Verge`
  - Fallback to proxy retry on update failure
  - Supported async port lookup and saving, ports support random generation
  - Modified port detection range to `1111-65536`
  - Optimized saving mechanism using smooth functions to prevent lag
- **Configuration & Security:**
  - Auto-complete missing `secret` to `set-your-secret`
  - Force Mihomo config to complete `external-controller-cors` (disabled cross-origin by default, restricted to local access)
  - Optimized window permissions and state initialization logic
  - Network latency tests replaced with HTTPS: `https://cp.cloudflare.com/generate_204`
  - Optimized IP info fetching, adding deduplication and polling check algorithms
- Synchronously fixed translation errors and inconsistencies, optimizing overall language experience
- Strengthened page stability after language switching to avoid loading anomalies

### 🗑️ Removed

- Window state manager (upstream issues)
- WebDAV cross-platform backup restoration limits

---

## v2.2.3

#### Known Issues

- Only briefly tested under the Gnome desktop environment on Ubuntu 22.04/24.04 and Fedora 41. Compatibility with other Linux distributions is not guaranteed, and adaptation/tuning will follow in the future.
- Recommended icon size for macOS custom icons and speed display is 256x256. Other sizes may cause abnormal icons and speed display spacing.
- On macOS, the background wallpaper is mainly light, causing the Tray icon to flicker when it is dark; color Tray speed indicator color is light.
- Clash Verge Rev memory usage under Linux is significantly higher than on Windows / macOS.

### 2.2.3 compared to 2.2.2

#### Fixed:

- Homepage "Current Proxy" CPU usage too high due to repeated refreshes
- "Autostart" and "DNS Override" switch jumping issues
- Custom tray icon failing to apply changes
- macOS custom tray icon display speed icon-to-text spacing too large
- macOS tray rate display incomplete
- Linux failing to launch Mihomo kernel in system service mode
- Async operations to avoid crashes from fetching system info and switching proxy modes
- Page rendering error caused by identical node names
- URL Schemes truncation issue
- Homepage traffic stats card using better timestamp ranges
- Silent start unable to trigger auto lightweight mode timer

#### Added:

- Mihomo(Meta) Kernel upgraded to v1.19.4
- Clash Verge Rev no longer strongly depends on system service and admin privileges
- Supported choosing between Sidecar (user space) mode or installing service based on user preference
- Added error prompts when loading initial config to prevent switching to incorrect subscriptions
- Detects if running as administrator, if so, prompts that autostart cannot be enabled
- Proxy groups show node count
- Unified operation mode detection, supporting enabling TUN mode in admin mode
- Tray proxy mode switching automatically disconnects previous connections based on settings
- Fallback to Clash kernel proxy retry on subscription fetch failure

#### Removed:

- Real-time window size and position saving. This feature might cause window size/position anomalies and needs further observation.

#### Optimized:

- Refactored backend kernel management logic, more lightweight and effective management, improving performance and stability
- Unified frontend data refresh, optimizing data fetching and refresh logic
- Optimized homepage traffic chart code, adjusting margins
- macOS tray rate display style and update logic
- Homepage only displays traffic chart area when traffic chart is present
- Updated default DNS override config
- Removed test directories, simplifying resource initialization

---

## v2.2.2

**Release Codename: Tuo**

Special thanks to @Tunglies for major contributions to Verge backend refactoring and performance optimization!

Codename meaning: This release represents a significant expansion in features. The new homepage design brings a brand new interactive experience. The DNS override function enhances network control. The unlock test page facilitates content access freedom. The lightweight mode provides flexible choices. In addition, new features like macOS app menu integration, sidecar mode, and diagnostic info export enrich user scenarios. These additions significantly expand the functional boundaries of Clash Verge, providing users with more powerful tools and possibilities.

#### Known Issues

- Only briefly tested under the Gnome desktop environment on Ubuntu 22.04/24.04 and Fedora 41. Compatibility with other Linux distributions is not guaranteed, and adaptation/tuning will follow in the future.

### 2.2.2 compared to 2.2.1

#### Fixed:

- Black dialog popups issue (triggered by service crash reinstall mechanism)
- Hide Dock icon after entering lightweight mode on macOS
- Added missing tray translation for lightweight mode
- Linux window border cutting issue

#### Added:

- Enhanced service detection and reinstall logic
- Enhanced kernel and service keep-alive mechanism
- Added zombie process cleanup mechanism under service mode
- Added auto rollback to user space mode after multiple service mode failures

### 2.2.1 compared to 2.2.0

#### Fixed:

1. **Homepage**
   - Fixed Direct mode homepage failing to render
   - Fixed homepage entering lightweight mode causing ClashVergeRev to exit from tray
   - Fixed inaccurate system proxy flag detection
   - Fixed incorrect system proxy address
   - Removed redundant transitions in proxy mode switching
2. **System**
   - Fixed macOS unable to use shortcuts to copy/paste/select subscription URLs
   - Fixed proxy port settings synchronization
   - Fixed Linux unable to communicate with Mihomo core and ClashVergeRev service
3. **UI**
   - Fixed connection detail card not following theme color
4. **Lightweight Mode**
   - Fixed macOS lightweight mode Dock icon failing to hide

#### Added:

1. **Homepage**
   - Text truncation when homepage text is too long
2. **Lightweight Mode**
   - Added tray entrance support for lightweight mode
   - Added hotkey support for entering lightweight mode
3. **System**
   - Always attempt to ensure both ClashVergeRev and Mihomo run when operating Mihomo
   - Under service mode, scan and terminate other existing kernel processes during Mihomo kernel startup to prevent kernel freeze issues
4. **Tray**
   - Added option to hide tray icon display when macOS tray rate display is enabled

---

## v2.2.0

#### Features

1. **Homepage**
   - Added homepage, changing default startup page to homepage
   - Homepage traffic card displays upload/download names
   - Homepage supports lightweight mode switching
   - Traffic stats data persistently saved
   - Restricted homepage profile card URL length
2. **DNS settings & override**
   - Added DNS override feature
   - Enabled DNS override by default
3. **Unlock tests**
   - Added unlock test page
4. **Lightweight Mode**
   - Added lightweight mode and settings
   - Added auto lightweight mode timer
5. **System Support**
   - Mihomo(meta) kernel upgraded to v1.19.3
   - macOS supports CMD+W to close window
   - Added macOS application menu
   - Added administrator privilege prompts when installing service under macOS
   - Added sidecar (user space core start) mode
6. **Others**
   - Enhanced latency test logs and error handling
   - Added diagnostic info export
   - Added proxy commands

---

## v1.7.7

### Bug Fixes

- Fixed importing subscription not auto-reloading (nodes not displaying)
- Fixed Windows tray tooltip text exceeding limits under English locale

---

## v1.7.6

### Notice

- Clash Verge Rev has entered a stable cycle; future updates will focus on bug fixes and regular kernel upgrades.

### Features

- Meta(mihomo) kernel upgraded to v1.18.7
- UI details adjustments
- Optimized service mode install logic
- Removed redundant console logs
- Auto select the first subscription

### Bug Fixes

- Fixed service mode installation issues
- Fixed macOS system proxy bypass CIDR filter
- Fixed 32-bit upgrade URL
- Fixed different groups URL test address configurations being invalid
- Fixed hostname parameter under Web UI

---

## v1.7.5

### Features

- Display LAN IP address info
- Copy environment variables directly on settings page
- Optimized service mode install logic

### Performance

- Optimized subscription switching speed
- Optimized port changing speed

### Bug Fixes

- Adjusted macOS tray icon size
- Trojan URI parsing error
- Card drag display layer error
- Proxy bypass formatting check error
- macOS editor maximize failure
- macOS service installation failure
- Changing window size causing crash issue

---

## v1.7.3

### Features

- Supported visual editing of subscription proxy groups
- Supported visual editing of subscription nodes
- Supported visual editing of subscription rules
- Extension scripts support subscription name parameter `function main(config, profileName)`

### Bug Fixes

- Proxy bypass formatting check error

---

## v1.7.2

### Break Changes

- Please make sure to re-import all subscriptions after updating, including Remote and Local.
- This version refactored Merge/Script. Please backup custom Merge and Script before updating.
- Merge renamed to `Extension Config`, split into `Global Extension Config` and `Subscription Extension Config`. Global extension config applies to all subscriptions, subscription extension config only applies to associated subscriptions.
- Script renamed to `Extension Script`, similarly split.
- Subscription extension config is accessed via subscription right-click menu.
- Execution priority: Global Extension Config -> Global Extension Script -> Subscription Extension Config -> Subscription Extension Script.
- Extension config removed `prepend/append` capability; please use Right-click subscription -> `Edit Rules` / `Edit Nodes` / `Edit Proxy Groups` instead.
- macOS users please reinstall service mode after updating.

### Features

- Upgraded kernel to v1.18.6
- Removed kernel authorization, replaced with service mode implementation
- Auto-fill local subscription name
- Added major update handling logic
- Subscriptions individually specify extension config/script (requires re-importing)
- Added visual rules editor (requires re-importing)
- Editor added toolbar buttons (Format, Maximize/Minimize)
- WEBUI uses the latest metacubex version and resolves auto-login issue
- Disabled some WebView2 shortcuts
- Hotkey configuration added connector `+` sign
- Added some floating tooltips for explanations
- When log level is `Debug` (requires restart to take effect), support clicking memory for active memory reclamation (green text)
- Added Telegram channel link in the top-right of settings page
- Various detail optimizations and UI performance improvements

### Bug Fixes

- Fixed proxy bypass formatting checks
- Close processes by process name
- Restore DNS settings when exiting software
- Fixed update interval failing to save when creating local subscriptions
- Connection page column width cannot be adjusted

---

## v1.7.1

### Break Changes

- Please make sure to re-import all subscriptions after updating, including Remote and Local.
- This version refactored Merge/Script. Please backup custom Merge and Script before updating.
- Merge renamed to `Extension Config`, split into `Global Extension Config` and `Subscription Extension Config`. Global extension config applies to all subscriptions, subscription extension config only applies to associated subscriptions.
- Script renamed to `Extension Script`, similarly split.
- Subscription extension config is accessed via subscription right-click menu.
- Execution priority: Global Extension Config -> Global Extension Script -> Subscription Extension Config -> Subscription Extension Script.
- Extension config removed `prepend/append` capability; please use Right-click subscription -> `Edit Rules` / `Edit Nodes` / `Edit Proxy Groups` instead.
- macOS users please reinstall service mode after updating.

### Features

- Upgraded kernel to v1.18.6
- Removed kernel authorization, replaced with service mode implementation
- Auto-fill local subscription name
- Added major update handling logic
- Subscriptions individually specify extension config/script (requires re-importing)
- Added visual rules editor (requires re-importing)
- Editor added toolbar buttons (Format, Maximize/Minimize)
- WEBUI uses the latest metacubex version and resolves auto-login issue
- Disabled some WebView2 shortcuts
- Hotkey configuration added connector `+` sign
- Added some floating tooltips for explanations
- When log level is `Debug` (requires restart to take effect), support clicking memory for active memory reclamation (green text)
- Added Telegram channel link in the top-right of settings page
- Various detail optimizations and UI performance improvements

### Bug Fixes

- Fixed proxy bypass formatting checks
- Close processes by process name
- Restore DNS settings when exiting software
- Fixed update interval failing to save when creating local subscriptions
- Connection page column width cannot be adjusted

---

## v1.7.0

### Break Changes

- This version refactored Merge/Script. Please backup custom Merge and Script before updating.
- Merge renamed to `Extension Config`, split into `Global Extension Config` and `Subscription Extension Config`.
- Script renamed to `Extension Script`, similarly split.
- Execution priority: Global Extension Config -> Global Extension Script -> Subscription Extension Config -> Subscription Extension Script.
- macOS users please reinstall service mode after updating.

### Features

- Removed kernel authorization, replaced with service mode implementation
- Auto-fill local subscription name
- Added major update handling logic
- Subscriptions individually specify extension config/script (requires re-importing)
- Added visual rules editor (requires re-importing)
- Editor added toolbar buttons (Format, Maximize/Minimize)
- WEBUI uses the latest metacubex version and resolves auto-login issue
- Disabled some WebView2 shortcuts
- Hotkey configuration added connector `+` sign
- Added some floating tooltips for explanations
- When log level is `Debug` (requires restart to take effect), support clicking memory for active memory reclamation (green text)
- Added Telegram channel link in the top-right of settings page

### Bug Fixes

- Fixed proxy bypass formatting checks
- Close processes by process name
- Restore DNS settings when exiting software
- Fixed update interval failing to save when creating local subscriptions
- Connection page column width cannot be adjusted

---

## v1.6.6

### Features

- macOS App signing
- Removed AppImage
- Added download button to app update dialog
- Keep default values when setting system proxy bypass
- System proxy bypass configuration format check

### Bug Fixes

- macOS proxy group icon failing to display
- Missing RPM package dependencies

---

## v1.6.5

### Features

- Added RPM package support
- Optimized details

### Bug Fixes

- macOS 10.15 blank editor issue
- macOS low version startup white screen issue

---

## v1.6.4

### Features

- System proxy supports PAC mode
- Allow closing unused ports
- Use new app icon
- macOS supports toggling tray icon monochrome/color mode
- CSS injection supports editing via editor
- Optimized proxy group list performance
- Optimized traffic graph performance
- Supported Persian language

### Bug Fixes

- Slow TUN activation after killing core
- Use default values when proxy bypass is empty
- Unable to read clipboard content
- Windows overwrite installation core occupation issue

---

## v1.6.2

### Features

- Supported local file drag-and-drop import
- Re-supported 32-bit CPU
- Added built-in Webview2 version
- Optimized Merge logic, supporting deep merge
- Removed prepend/append-provider fields in Merge config
- Supported updating stable version kernel

### Bug Fixes

- macOS DNS restoration failure
- CMD environment variable format error
- Compatibility issue with Linux NVIDIA GPUs
- Modifying TUN settings not taking effect immediately

---

## v1.6.1

### Features

- Hovering displays current subscription name
- Log filtering supports regular expressions
- Upgraded Clash kernel to v1.18.4

### Bug Fixes

- Fixed system proxy unable to enable under Linux KDE environments
- Adjusted window maximization icon
- Modified macOS tray click behavior (left click menu, right click event)
- Fixed macOS service mode installation failure

---

## v1.6.0

### Features

- Meta(mihomo) kernel rolled back to v1.18.1 (newer kernel has hy2 protocol bugs, will update after fix)
- Various UI detail adjustments
- Added service mode under Linux
- Added right-click on subscription card to open airport homepage
- url-test supports manual selection, node group fixed nodes display with badges
- Clash config and Merge config provide JSON Schema syntax support; connection page adjustments
- Modified Merge default config content
- Modified TUN mode default MTU to 1500; old versions upgrading should "reset to default" under TUN settings
- Use npm to install meta-json-schema
- Updated some translations
- Supported taskbar icon in ico format

### Bug Fixes

- Fixed system proxy unable to enable under Linux KDE environments
- Fixed latency test animation issues
- Adjusted window maximization icon
- Fixed Windows service mode installation failure under certain situations
- UI detail fixes
- Fixed opening configuration file using default editor
- Fixed kernel updates in specific directories
- Fixed service mode installation directory issue
- Fixed issue with deleting profile "update interval"

---

## v1.5.11

### Features

- Meta(mihomo) kernel upgraded to v1.18.2

### Bug Fixes

- Upgrade icon unable to be clicked
- Check if installation directory is empty during uninstall
- Overlapping icons in proxy UI

---

## v1.5.10

### Features

- Optimized Linux tray menu display
- Added transparent proxy port settings
- Confirmation before deleting subscriptions

### Bug Fixes

- Delete macOS Dock icon
- Windows service logs not cleaned up
- macOS unable to enable system proxy

---

## v1.5.9

### Features

- Cache proxy group icons
- Use `boa_engine` instead of `rquickjs`
- Supported Linux armv7

### Bug Fixes

- Windows first install unable to click
- Windows touch screen unable to drag
- `REJECT-DROP` rule list color
- macOS Dock does not display icon
- macOS custom font invalid
- Avoid fetching subscriptions with empty UA

---

## v1.5.8

### Features

- Optimized UI details
- Draw window rounded corners on Linux
- Enabled DevTools

### Bug Fixes

- Fixed macOS kernel crash when enabling Tun mode

---

## v1.5.7

### Features

- Optimized various UI details
- Provided tray menu icon style options (monochrome/color/disabled)
- Added auto check updates switch
- macOS auto modifies DNS when enabling Tun mode
- Adjusted draggable area (attempting to fix touch screen drag issues)

---

## v1.5.6

### Features

- Brand new exclusive Verge rev UI and detail adjustments
- Provided switch to allow invalid certificates
- Removed unnecessary hotkeys
- Added update animation for Providers
- Merge supports Providers
- Replaced paste button in subscription box, removed default "Remote File" profile name
- Added node display in connection menu

### Bug Fixes

- Linux image display error

---

## v1.5.4

### Features

- Supported custom tray icons
- Supported disabling proxy group icons
- Proxy groups show current proxy
- Changed "Open Panel" hotkey to "Open/Close Panel"

---

## v1.5.3

### Features

- Added reset button for Tun settings

### Bug Fixes

- Tun settings display error
- Modified some default values
- Do not modify startup settings at launch

---

## v1.5.2

### Features

- Supported custom latency test timeout
- Optimized Tun related settings

### Bug Fixes

- Merge operation error
- Restart service after installation
- Fixed autostart failure when starting with administrator privileges

---

## v1.5.1

### Features

- Save window maximized status
- Proxy Provider shows count
- No longer provide 32-bit installation packages (32-bit has various issues like unable to enable TUN mode)

### Bug Fixes

- Optimized setting names
- Fixed proxy group display error when customizing GLOBAL proxy group

---

## v1.5.0

### Features

- Removed Clash field filtering feature
- Added settings for socks port and http port
- Upgraded kernel to v1.18.1

### Bug Fixes

- Fixed 32-bit version unable to display traffic info

---

## v1.4.11

### Break Changes

- This version changed the Windows installation package install mode. You need to uninstall and manually install, otherwise it won't install in the correct location.

### Features

- Optimized system proxy enabling code, solving lag in rare scenarios
- Added macOS debug logs to troubleshoot future macOS system proxy issues
- Terminate background GUI synchronously when macOS GUI is closed

### Bug Fixes

- Resolved file occupation issue during auto-update
- Resolved system proxy enabling failure in rare scenarios
- Removed redundant kernel code

---

## v1.4.10

### Features

- Added exit button in settings
- Supported custom app startup page
- Show subscription info in Proxy Provider page
- Optimized Provider support

---

## v1.4.9

### Features

- Supported running script at startup
- Supported proxy group icons display
- Added test page

### Bug Fixes

- Connection page time sorting error
- Connection page table width optimized

---

## v1.4.8

### Features

- Connection page total traffic display

### Bug Fixes

- Connection page data sorting error
- Setting update interval invalid when creating new subscriptions
- Windows dial-up network unable to set system proxy
- Windows system proxy switch delay (using registry instead)
- Removed invalid background blur option

---

## v1.4.7

### Features

- Windows Portable version disables in-app updates
- Supported proxy group Hidden option
- Supported URL Scheme (macOS & Linux)

---

## v1.4.6

### Features

- Upgraded Clash Meta(mihomo) kernel to v1.18.0
- Supported URL Scheme (temporarily Windows only)
- Added window pin button
- UI optimization adjustments

### Bug Fixes

- Fixed some compile errors
- Error fetching subscription name
- Subscription info parsing error

---

## v1.4.5

### Features

- Updated macOS tray icon styles

### Bug Fixes

- Fixed Windows unable to overwrite `clash-verge-service.exe` during update (requires reinstalling service once)
- Window maximization button changes
- Window size saving error
- Copy env variable type unable to switch
- Crash under certain circumstances
- Inability to import some subscriptions

---

## v1.4.4

### Features

- Supported Windows aarch64(arm64) version
- Supported one-click update of GeoData
- Supported one-click update of Alpha kernel
- macOS supports displaying different tray icons for system proxy
- Linux supports displaying different tray icons for system proxy
- Optimized copy env variables logic

### Bug Fixes

- Modified PID file path

### Performance

- Optimized window creation speed

---

## v1.4.3

### Break Changes

- Changed config file path to standard directories (cleaner uninstall)
- Changed appid to `io.github.clash-verge-rev.clash-verge-rev`
- Recommended to uninstall old version before installing new version

### Features

- Removed page transitions
- Changed Tun mode tray icon color
- Portable version defaults to using current directory as configuration directory
- Hidden Clash field options when disabling Clash fields filter
- Optimized cursor style when dragging

### Bug Fixes

- Fixed update failure on Windows due to kernel not closing
- Fixed file opening error
- Fixed URL import unable to fetch Chinese config names
- Fixed alpha kernel unable to display memory info

---

## v1.4.2

### Features

- Upgraded clash meta core to mihomo v1.17.0
- Support both clash meta stable release and prerelease-alpha release
- Fixed system proxy configuration under Windows dial-up connections
- Support new clash fields
- Support random mixed port
- Added Windows x86 and Linux armv7 support
- Support disabling tray click events
- Added download progress for updater
- Support drag to reorder profiles
- Embedded emoji fonts
- Updated dependencies
- Improved UI style

---

## v1.4.1

### Features

- Upgraded clash meta core to latest Mihomo (2023.11.23)
- Removed clash core UI
- Improved UI
- Changed logo to original

---

## v1.4.0

### Features

- Upgraded clash meta core to latest Mihomo
- Removed clash core, no longer maintained
- Merged Clash Nyanpasu changes
- Removed delay display different colors
- Use Meta Country.mmdb
- Updated dependencies
- Small improvements

---

## v1.3.8

### Features

- Upgraded clash meta core
- Added default valid keys
- Adjusted delay display intervals and colors

### Bug Fixes

- Fixed connection page undefined exception

---

## v1.3.7

### Features

- Upgraded clash and clash meta core
- Profiles page added paste button
- Subscription URL text field supports multiple lines
- Set minimum window size
- Added check for updates buttons
- Added open dashboard to hotkey list

### Bug Fixes

- Fixed profiles page undefined exception

---

## v1.3.6

### Features

- Added Russian translation
- Support to show connection details
- Support clash meta memory usage display
- Support proxy provider update UI
- Updated geo data files from meta repo
- Adjusted settings page

### Bug Fixes

- Center window when out of screen bounds
- Use `sudo` when `pkexec` is not found under Linux
- Reconnect WebSockets on window focus

### Notes

- Linux installation packages are built on Ubuntu 20.04.

---

## v1.3.5

### Features

- Upgraded clash core

### Bug Fixes

- Fixed blurry system tray icon under Windows
- Fixed wintun.dll not found under Windows
- Fixed clash core not found under macOS and Linux

---

## v1.3.4

### Features

- Upgraded clash and clash meta core
- Optimized traffic graph high CPU usage when window is hidden
- Use polkit to elevate permissions on Linux
- Support app log level configuration
- Support copying environment variables
- Overwrite resource files based on modification time
- Save window size and position

### Bug Fixes

- Removed fallback group select status
- Enabled context menu on editable elements under Windows

---

## v1.3.3

### Features

- Upgraded clash and clash meta core
- Show tray icon variants under different system proxy statuses on Windows
- Close all connections when mode changes

### Bug Fixes

- Encode controller secret into URI
- Error boundary for each page

---

## v1.3.2

### Features

- Upgraded clash and clash meta core

### Bug Fixes

- Fixed URL import issues
- Fixed profile undefined issues

---

## v1.3.1

### Features

- Upgraded clash and clash meta core

### Bug Fixes

- Fixed URL opening issues
- Fixed AppImage path panic
- Fixed macOS root permission grant
- Fixed Linux system proxy default bypass

---

## v1.3.0

### Features

- Upgraded clash and clash meta core
- Support opening directories from tray
- Support updating all profiles with one click
- Support granting root permissions to clash core (Linux, macOS)
- Support enabling/disabling clash fields filter to experience the latest features of Clash Meta

### Bug Fixes

- Added openssl dependency to deb packages on Linux
- Fixed AppImage autostart path on Linux
- Fixed retrieving default network service on macOS
- Removed ESC key listener in macOS, cmd+w instead
- Fixed infinite retries on WebSocket errors

---

## v1.2.3

### Features

- Upgraded clash core
- Adjusted macOS window style
- Profile supports UTF8 with BOM

### Bug Fixes

- Fixed selected proxy
- Fixed error logs

---

## v1.2.2

### Features

- Upgraded clash meta core
- Recover clash core after panic
- Use system window decorations on Linux

### Bug Fixes

- Flush system proxy settings on Windows
- Fixed parse log panic
- Fixed UI bugs

---

## v1.2.1

### Features

- Upgraded clash version
- Proxy groups support multiple columns
- Optimized UI

### Bug Fixes

- Fixed UI WebSocket connection
- Adjusted delay check concurrency
- Avoid setting login items repeatedly on macOS

---

## v1.2.0

### Features

- Upgraded clash meta version
- Support changing external-controller
- Support changing default latency test URL
- Close all connections when proxy or profile changes
- Verify config using the core
- Increased program robustness
- Optimized Windows service mode (requires re-installation)
- Optimized UI

### Bug Fixes

- Invalid hotkey causing panic
- Invalid theme setting causing panic
- Fixed other glitches

---

## v1.1.2

### Features

- System tray follows i18n
- Changed proxy group UI of global mode
- Support updating profile with system proxy/clash proxy
- Check remote profile more strictly

### Bug Fixes

- Use app version as default User Agent
- Clash does not exit in service mode
- Reset system proxy on app exit
- Fixed other glitches

---

## v1.1.1

### Features

- Optimized clash config feedback
- Hidden macOS Dock icon
- Use clash meta compatible version under Linux

### Bug Fixes

- Fixed other glitches

---

## v1.1.0

### Features

- Added rule page
- Supported proxy providers delay check
- Added proxy delay check loading status
- Supported hotkey/shortcut management
- Supported displaying connections data in table layout

### Bug Fixes

- Supported yaml merge keys in clash config
- Detect network interface and configure system proxy on macOS
- Fixed other glitches

---

## v1.0.6

### Features

- Upgraded clash and clash.meta cores

### Bug Fixes

- Only script profiles display console
- Automatic configuration updates on demand at launch

---

## v1.0.5

### Features

- Reimplemented profile enhanced mode with quick-js
- Optimized runtime config generation process
- Supported Web UI management
- Supported clash field management
- Supported viewing the runtime config
- Adjusted some page styles

### Bug Fixes

- Fixed silent start
- Fixed incorrectly resetting system proxy on exit

---

## v1.0.4

### Features

- Upgraded clash core and clash meta version
- Support switching clash mode on system tray
- Theme mode supports following system

### Bug Fixes

- Config load error on first use

---

## v1.0.3

### Features

- Save some states such as URL test, filter, etc.
- Upgraded clash core and clash-meta core
- New icon for macOS

---

## v1.0.2

### Features

- Supported switching clash cores
- Supported releasing UI processes
- Supported script mode settings

### Bug Fixes

- Fixed service mode bug on Windows

---

## v1.0.1

### Features

- Adjusted default theme settings
- Reduced GPU usage of traffic graph when hidden
- Supported more remote profile response header settings
- Check remote profile data format when imported

### Bug Fixes

- Service mode installation and startup issues on Windows
- Fixed launch panic on some Windows systems

---

## v1.0.0

### Features

- Upgraded clash core
- Optimized traffic graph animation
- Supported interval update profiles
- Supported service mode on Windows

### Bug Fixes

- Reset system proxy when exiting from Dock on macOS
- Adjusted clash DNS configuration processing strategy

---

## v0.0.29

### Features

- Sort proxy nodes
- Custom proxy test URL
- Logs page filter
- Connections page filter
- Default User Agent for subscription
- System tray added TUN mode toggle
- Enabled changing the config directory (Windows only)

---

## v0.0.28

### Features

- Enabled using clash config fields in UI

### Bug Fixes

- Removed character anomalies
- Fixed some icon colors

---

## v0.0.27

### Features

- Supported custom theme colors
- TUN mode settings control the final configuration

### Bug Fixes

- Fixed transition flickers on macOS
- Reduced proxy page renders

---

## v0.0.26

### Features

- Silent start
- Profile editor
- Profile enhanced mode supports more fields
- Optimized profile enhanced mode strategy

### Bug Fixes

- Fixed CSP restriction on macOS
- Window controllers on Linux

---

## v0.0.25

### Features

- Upgraded clash core version

### Bug Fixes

- App updater error
- Display window controllers on Linux

### Notes

- If you cannot update the app properly, please download the latest version from GitHub releases.

---

## v0.0.24

### Features

- Connections page
- Added wintun.dll (Windows)
- Supported creating local profiles with selected files (Windows)
- System tray enabled setting system proxy

### Bug Fixes

- Open directory error
- Auto-launch path on Windows
- Fixed some clash config errors
- Reduced the impact of the enhanced mode

---

## v0.0.23

### Features

- i18n support
- Remote profile User Agent support

### Bug Fixes

- Clash config file case ignored
- Clash `external-controller` only port
