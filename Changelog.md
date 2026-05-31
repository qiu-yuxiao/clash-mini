## v1.1.8

### 🐞 修复问题

- 修复 BUG-028：开机自启动客户端在网络尚未就绪更新失败时，增加 2 分钟自动重试机制，并在订阅无更新间隔时默认使用 24 小时兜底。
- 修复 BUG-034：切换或激活订阅时启动自适应测速轮询，在 30 秒内扫描到至少 5 个可用节点时立即切至最快，超时降级处理，并自动过滤广告和提示节点。

## v2.5.2

### 🐞 修复问题

- macOS 托盘速率可能的样式错误

<details>
<summary><strong> ✨ 新增功能 </strong></summary>

- 增加 TrustTunnel, OpenVPN, Tailscale, GostRelay 节点显示支持

</details>

<details>
<summary><strong> 🚀 优化改进 </strong></summary>

- 关闭 autofill 弹出窗口

</details>
