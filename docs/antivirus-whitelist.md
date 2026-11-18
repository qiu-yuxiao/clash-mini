# 轻量模式防误杀指南（360 等安全软件处理说明）

> 本文内容与随 Clash Mini 安装包/便携包同步释放至安装目录的 **`轻量模式防误杀指南.txt`**（可通过客户端顶栏「`?` 帮助菜单 → 🛡️ 轻量模式防误杀指南」直接打开）保持一致，供仓库及 Wiki 读者查阅。

## 一、为什么 Clash Mini 会被 360 莫名其妙强行关闭？

Clash Mini 在关闭主窗口进入**轻量模式**时，会销毁 WebView2 前端界面进程以释放约 30～40MB 内存，仅保留系统托盘图标与代理内核（`mini-mihomo.exe`）常驻后台维持不断网不断流与 15 秒健康监测。（注：开启 TUN 模式时 `mini-mihomo.exe` 以系统服务运行，通常不受影响，但桌面会话内的 `clash-mini.exe` 仍可能被清理退出。）

经对 360 安全卫士性能引擎（`360tray.exe` 加载 `safemon\Netm.tpi` 与 `netmon\360perfoptm2.dll`）实证分析，360 普通查杀的「信任区」对性能清理**完全无效**。后台存在**两套独立强杀机制**，分别读取不同的白名单文件：

1. **机制一：关窗残留清理（`DoOptmGroupResidual`）**
   - **触发条件**：全局记录「曾经打开过可见窗口、随后又关闭窗口」的后台程序，一旦机器持续高 CPU 负载便将其视为“关窗残留”强行终止。
   - **白名单文件**：**仅读取** `netmon\whiteinternal.ini`。
2. **机制二：高内存 / 智能加速与游戏模式清理（`FastOptm` / `DoOptm` / `FastOptmForGameMode`）**
   - **触发条件**：当系统内存占用冲高（如 `85%～90%+`）或触发智能加速/游戏模式清理时，**即使 Clash Mini 主窗口正开着（从未关闭窗口）也会被直接强杀**。
   - **白名单文件**：**完全不读** `whiteinternal.ini`，而是**仅读取** `netmon\useroptm2.ini` 与 `netmon\useroptm_game2.ini` 中的 `check=0` 标记。

---

## 二、方案一：一劳永逸根治方案（Windows 安全模式修改 3 个底层白名单文件，推荐）

`C:\Program Files (x86)\360\360Safe\netmon\` 目录受 `360SelfProtection` 内核驱动写保护，正常 Windows 下无法直接修改（拒绝访问），必须在 **Windows 安全模式**下用记事本修改一次（若 360 安装在其他目录，请替换为实际路径）：

1. **进入 Windows 安全模式**：
   按住键盘 `Shift` 键不放，点击 Windows「开始菜单 → 电源 → 重启」→ 进入蓝屏恢复菜单后依次选择「疑难解答 → 高级选项 → 启动设置 → 重启」→ 重启界面按数字键 `4`（启用安全模式）。
2. **修改机制一白名单（`whiteinternal.ini`）**：
   用记事本打开：
   ```text
   C:\Program Files (x86)\360\360Safe\netmon\whiteinternal.ini
   ```
   找到 `[paths]` 段并在末尾追加以下两行（若已有 `path1=`、`path2=`，请顺延编号如 `path3=`、`path4=`；若安装在非默认路径，保留「最后一级安装目录名\程序名」格式即可，例如 `D:\Tools\Clash Mini\clash-mini.exe` 同样写作 `Clash Mini\clash-mini.exe`）：
   ```ini
   path1=Clash Mini\clash-mini.exe
   path2=Clash Mini\mini-mihomo.exe
   ```
3. **修改机制二白名单（`useroptm2.ini` 与 `useroptm_game2.ini`）**：
   用记事本分别打开同目录下的以下**两个文件**（若某文件不存在可直接新建）：
   ```text
   C:\Program Files (x86)\360\360Safe\netmon\useroptm2.ini
   C:\Program Files (x86)\360\360Safe\netmon\useroptm_game2.ini
   ```
   在**两个文件的末尾**分别追加以下 6 行（若已存在同名段落且为 `check=1`，直接改为 `check=0`）：
   ```ini
   [clash-mini.exe]
   check=0
   [clash mini_0]
   check=0
   [mini-mihomo.exe]
   check=0
   ```
4. **保存并正常重启电脑**：
   保存上述 3 个文件后正常重启进入 Windows，两套防杀白名单即永久生效，此后无论开关主窗口还是遭遇高内存/高 CPU 满载编译，均不会被 360 误杀。

---

## 三、方案二：暂不进安全模式的日常减轻方案（开启「静默启动」）

1. 在 Clash Mini「设置 → 基本设置」中开启 **「静默启动」**。
2. 从 `v3.5.2` 起，只要启用「静默启动」且已有订阅配置，无论开机自启还是手动双击桌面/开始菜单图标，Clash Mini 都会直接静默进入系统托盘而不创建或弹出主窗口（手动启动时仅弹出系统托盘气泡提示已在后台运行），从而避开机制一（关窗残留记录）。
3. **局限性提示**：一旦左键点开过主窗口，或系统内存冲高触发机制二（`FastOptm` / `DoOptm`），仍可能被 360 清理；若被清理，再次双击图标即可无窗静默恢复托盘驻留。如需彻底根治请按「方案一」一次配齐全部 3 个白名单文件。

---

## 四、其他常见安全软件说明与技术边界

- **Windows Defender / 火绒等常规杀软**：不存在上述后台性能强杀机制；如遇常规误报，将 Clash Mini 安装目录（默认 `C:\Program Files\Clash Mini\`）或 `clash-mini.exe`、`mini-mihomo.exe` 加入其排除项即可。
- **技术边界声明**：Clash Mini 仅使用 Tauri 标准 `RunEvent::ExitRequested`（`prevent_exit`）维持托盘后台驻留，绝不尝试在代码层绕过、破坏或强改第三方安全软件文件；上述安全模式配置完全由用户自主决定与执行。
