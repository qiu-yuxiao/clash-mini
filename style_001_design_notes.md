# STYLE-001: Clash Mini 全场景 3D 物理拟物设计规范与参数说明

本文件详细记录了针对 `STYLE-001`（全场景 3D 立体感与物理凹凸质感微调优化）的视觉调研方案与具体 CSS 技术参数规范，用作后续动工实现的技术指导。

---

## 🎨 一、 核心设计理念

本优化的核心是**拟物化（Skeuomorphism）**与**物理世界的光影映射**。所有界面元素都应当被赋予物理厚度，并遵循以下两条物理法则：
1. **单一光源假设**：假定光源来自屏幕的**左上方**（朝向右下方照射）。因此，所有凸起或凹陷元素的左上边缘均为**高光（白光）**，右下边缘均为**暗影（黑影）**。
2. **状态深度联动**：所有阴影的模糊半径、厚度和发光强度必须乘以 `--depth-factor`（立体度因子）与 `--vibrancy-factor`（色彩霓虹因子）进行动态缩放。

---

## 🛠️ 二、 关键组件 3D 参数设计规范

### 1. ⌨️ 机械键盘级“实体按键式”按钮（3D Buttons）
为了模拟真实的物理键盘按键手感，按钮的设计包含三个状态的物理变形：

* **A. 默认常态（Normal State）**
  取消常规的 1px 平淡边框，改用立体厚底阴影层叠（Box Shadow）：
  ```css
  border: 1px solid rgba(0, 0, 0, 0.15);
  transform: translateY(0);
  box-shadow:
    /* 1. 物理底座厚度（深色实体厚度） */
    0 calc(3px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    /* 2. 底座投射到面板的软阴影 */
    0 calc(4px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.25),
    /* 3. 顶面边缘高光（左上内发光） */
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.4),
    /* 4. 顶面边缘暗面（右下内阴影） */
    inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.15);
  ```

* **B. 悬浮状态（Hover State）**
  按钮物理向上抬升，厚底拉长，软阴影扩散：
  ```css
  transform: translateY(-2px);
  box-shadow:
    /* 底座厚度拉长 */
    0 calc(5px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    /* 软阴影范围变大且更淡 */
    0 calc(7px * var(--depth-factor, 1.0)) calc(12px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.2),
    /* 高光微幅增强 */
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.5),
    inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.1);
  ```

* **C. 点击状态（Active/Pressed State）**
  模拟按钮被物理按压下沉，厚底消失，转化为物理下陷：
  ```css
  transform: translateY(3px); /* 下压距离等于常态下的底座厚度 */
  box-shadow:
    /* 厚底与软外影完全消失 */
    0 0 0 0 transparent,
    /* 转化为内部物理下凹阴影 */
    inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.35);
  ```

* **过度动效（Transition）**：
  采用具有轻微物理回弹感的贝塞尔曲线过渡：`transition: all 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275)`。

---

### 2. 🎚️ 滑轨与滑块的“凹槽与金属滚珠”设计（Sliders & Switches）

* **A. 滑动轨道 / 开关底轨（Track/Groove）**
  设计为向面板下方**深深内切**的物理滑槽（Groove）：
  ```css
  height: 14px;
  border-radius: 7px;
  background: var(--theme-track-bg-dark); /* 暗灰中性色轨底 */
  box-shadow:
    /* 左上方注入内阴影模拟槽壁阴影 */
    inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.4),
    /* 右下方注入微弱反光模拟凹槽外边缘 */
    inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(255, 255, 255, 0.08);
  ```

* **B. 滑块 / 开关滑珠（Thumb/Sphere）**
  滑块或开关的圆形按钮设计为凸起的 **3D 镜面反光球体**，模拟金属或高光玻璃材质：
  ```css
  width: 14px;
  height: 14px;
  border-radius: 50%;
  /* 左上方球状漫反射高光 gradient */
  background: radial-gradient(circle at 35% 35%, #ffffff 0%, var(--primary-color-main) 70%, var(--primary-color-dark) 100%);
  box-shadow:
    /* 滚珠在滑道上投射的微阴影 */
    0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.3),
    /* 边缘的金属外环倒角 */
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.6);
  ```

---

### 3. 📥 物理下凹输入框（Recessed Input Fields）
所有文字输入栏（如机场链接输入框、主页节点搜索框）采用**下凹刻槽（Chiseled Inner Bevel）**设计，使它们仿佛牢固内嵌在面板内：
```css
box-shadow:
  /* 内部下凹阴影 */
  inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.15),
  /* 右下外框微弱反光 */
  1px 1px 1px rgba(255, 255, 255, 0.05);
```
在 **Focus（聚焦）** 状态下，外框发光与 `--vibrancy-factor` 联动，外射微弱霓虹彩光：
```css
box-shadow:
  inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.2),
  0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.3 * var(--vibrancy-factor, 1.0)));
```

---

### 4. 🔲 凹槽滑槽分段选择器（Segmented Control）
对于“主题选择（系统/浅/深）”、“流量接管模式”等分段切换控件：
* **背景轨道（Outer Container）**：高度固定为 `26px`，整体设置 `box-shadow: inset 1.5px 1.5px 3px rgba(0,0,0,0.3)` 形成一个下凹的“物理面板刻槽”。
* **滑动指示器（Sliding Active Block）**：高度为 `22px`，设置为具有 3D 浮雕感的卡片，悬浮在凹槽之上，滑动时带有顺畅阻尼物理感。

---

### 5. 📊 底部双色 3D 浮雕数据小卡片（Metrics Cards）
主页最下边一行的 4 个小卡片（上传速度、上传总量、下载速度、下载总量）需要与整体的 3D 立体拟物风格深度统一，并根据上传/下载属性呈现不同的主题霓虹底色（上传为金色，下载为蓝色）：

* **A. 默认常态（Normal State）**
  卡片被渲染为向面板上方微凸的 3D 立体水晶板，左侧卡片仅有左圆角，右侧仅有右圆角，两两紧密贴合：
  ```css
  height: 22px;
  border: 1px solid rgba(var(--metrics-theme-color-rgb), calc(0.25 * var(--depth-factor, 1.0) + 0.18 * var(--vibrancy-factor, 1.0)));
  box-shadow:
    /* 1. 外围微弱的软阴影 */
    0 calc(2px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, calc(0.25 * var(--depth-factor, 1.0))),
    /* 2. 顶边左上白高光 Bevel */
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255, 255, 255, calc(0.6 * var(--depth-factor, 1.0))),
    /* 3. 底边右下深色 Bevel */
    inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, calc(0.15 * var(--depth-factor, 1.0))),
    /* 4. 底部的彩色霓虹背光 */
    0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--metrics-theme-color-rgb), calc(0.12 * var(--vibrancy-factor, 1.0)));
  ```

* **B. 悬浮状态（Hover State）**
  鼠标悬停在卡片上时，卡片整体产生浮空感（translateY 向上微抬，投影与彩色背光扩散）：
  ```css
  transform: translateY(-1.5px);
  filter: brightness(1.08); /* 边缘镜面高光微闪 */
  box-shadow:
    0 calc(4px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0, 0, 0, calc(0.35 * var(--depth-factor, 1.0))),
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255, 255, 255, calc(0.8 * var(--depth-factor, 1.0))),
    inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, calc(0.12 * var(--depth-factor, 1.0))),
    0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--metrics-theme-color-rgb), calc(0.22 * var(--vibrancy-factor, 1.0)));
  ```

---

## 📋 三、 后续动工实现计划

1. **统一按钮样式接口**：更新 `src/utils/button-styles.ts`，增加 `transform` 和 `boxShadow` 的机械按键（Thick Bevel）过渡与按下变形参数。
2. **覆盖 MUI 常规样式**：在 `layout.scss` 中增加全局的输入框、滑块 and 开关组件的 3D 立体感类样式，以便快速应用。
3. **参数自适应关联**：核对所有的立体感参数中均已加入并乘上了 `var(--depth-factor)` 和 `var(--vibrancy-factor)`，保证用户滑动设定页的立体度和霓虹度滑动条时，全场景样式能够平滑响应、实时缩放。
