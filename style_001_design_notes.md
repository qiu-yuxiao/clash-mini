# STYLE-001: Clash Mini 重度 3D 物理拟物设计规范与参数说明 (参考 AIMP Euphoria Lux NY 风格)

本文件详细记录了针对 `STYLE-001` 进行**重度 3D 立体感与凹凸物理凹槽质感强化**的技术参数规范，主要灵感来源于 AIMP 经典的 **Euphoria Lux** 系列拟物皮肤。

---

## 🎨 一、 核心设计理念

1. **固态挤压厚度（Solid Extrusion Thickness）**：
   抛弃单一的模糊阴影，改用**多层无模糊像素层叠**来模拟实体物理按键的侧边立体厚度（Extrusion）。通过多层 `calc(Xpx * var(--depth-factor))` 的 `box-shadow` 在不需要修改 HTML 高度或宽度的情况下，实现真正物理三维侧边。

2. **单一光源与边缘高光（Single Light Source & Edge Highlights）**：
   光源假设来自屏幕**左上方**。
   * **凸起元素**（按钮、卡片、滑块）的左上角边缘需要有明亮的高对比度内高光（White Highlight, `0.6` 到 `0.8` 不透明度）。
   * **凹陷元素**（输入框、滑轨、底槽）的下边缘（Bottom-Lip）需要有一条反射光线（Highlight Line, `0 1px 0 rgba(255,255,255,0.08)` / `0.8`），表示光源照在凹模底沿上。

3. **面材质渐变（Face Gradients）**：
   不可使用纯平的单色，所有可交互表面均必须使用带有轻微明暗弧度的线性渐变（Linear Gradient）或径面渐变（Radial Gradient），以模拟塑料或打磨金属表面的光泽感。

---

## 🛠️ 二、 关键组件 3D 参数设计规范

### 1. ⌨️ 实体按键式按钮（3D Buttons）
为了模拟机械控制台按键，按钮包含以下三个状态：

* **A. 默认常态（Normal State）**
  具有 5 层厚底固态阴影 + 1层软阴影 + 左上倒角高光：
  ```css
  background: linear-gradient(to bottom, var(--btn-face-light) 0%, var(--btn-face-dark) 100%);
  border: 1px solid rgba(0, 0, 0, 0.3); /* 高对比度物理边界 */
  box-shadow:
    /* 1. 五层固态底座厚度挤压 */
    0 calc(1px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(2px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(3px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(4px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(5px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    /* 2. 底座投影到面板的软阴影 */
    0 calc(5px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45),
    /* 3. 顶面边缘高光（左上内发光） */
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.65),
    /* 4. 顶面边缘暗面（右下内阴影） */
    inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.25);
  ```

* **B. 悬浮状态（Hover State）**
  按钮物理向上抬升 `translateY(-2px)`，固态厚度拉伸至 7 层，环境投影软化：
  ```css
  transform: translateY(-2px);
  box-shadow:
    /* 七层固态挤压 */
    0 calc(1px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(2px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(3px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(4px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(5px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(6px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    0 calc(7px * var(--depth-factor, 1.0)) 0 0 var(--theme-button-bevel-shadow-dark),
    /* 软环境投影扩散 */
    0 calc(7px * var(--depth-factor, 1.0)) calc(14px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.35),
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.8),
    inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.15);
  ```

* **C. 点击状态（Active/Pressed State）**
  按钮下沉 `translateY(4px)`，侧边挤压完全消失，转化为内部下陷阴影：
  ```css
  transform: translateY(4px);
  box-shadow:
    0 0 0 0 transparent,
    inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.55);
  ```

---

### 2. 🎚️ 深度刻槽与金属滑珠（Sliders & Switches）

* **A. 滑动轨道与开关滑道（Deep Grooves）**
  设计为向面板下方**深深内切**的物理凹轨，并带底沿高光线：
  ```css
  box-shadow:
    /* 左上方深内阴影 */
    inset calc(3px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.65),
    /* 下边缘反光亮线 */
    0 1px 0 rgba(255, 255, 255, 0.08); /* 亮色模式下使用 0.8 */
  ```

* **B. 开关滑珠（Metallic Sphere / LED Glow）**
  滑珠呈现强烈的**抛光镜面球体质感（Chrome Sphere）**：
  ```css
  /* 镜面球状径向渐变 */
  background: radial-gradient(circle at 35% 35%, #ffffff 0%, #4a5568 45%, #1a202c 100%);
  box-shadow:
    /* 物理投影 */
    0 calc(3px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.5),
    /* 边缘倒角高光 */
    inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.5);
  ```
  开启（Checked）时，滑珠转换为**自发光 LED 材质**：
  ```css
  background: radial-gradient(circle at 35% 35%, #ffffff 0%, var(--primary-main) 60%, var(--primary-dark) 100%);
  ```

---

### 3. 📥 物理刻模输入框（Deep Recessed Inputs）
所有文字输入框均呈现牢固镶嵌在主机箱面板上的凹槽感：
```css
box-shadow:
  /* 深层内阴影 */
  inset 0 calc(3.5px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.55),
  /* 底沿反光 */
  0 1px 0 rgba(255, 255, 255, 0.08);
```

---

### 4. 🔲 凹槽滑槽分段选择器（Segmented Controls）
* **外槽容器（Outer Container）**：应用重度刻凹深模阴影，表现得像一个金属或塑料切槽。
* **浮雕指示块（Active Keycap）**：滑动块必须像一块立体的物理浮块卡在轨道槽之上，具备 3px 高度的固态侧边：
  ```css
  box-shadow:
    /* 指示块 3 层固态凸起侧边 */
    0 calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(0, 0, 0, 0.25),
    0 calc(2px * var(--depth-factor, 1.0)) 0 0 rgba(0, 0, 0, 0.25),
    0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.4),
    /* 顶边边缘高光 */
    inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.3);
  ```

---

### 5. 📊 双色浮雕数据小卡片（Metrics Cards）
主页底部的流量数据小卡片，表现为 3D 亚克力或水晶材质拼接浮雕：
* **常态**：具有 2px 的挤压高度，并且在拼接的边缘应用相反方向的高对比高光与深影：
  ```css
  box-shadow:
    0 calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(0,0,0,0.2),
    0 calc(2.5px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
    inset 0 calc(1.2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.25),
    inset 0 calc(-1.2px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.4);
  ```
* **悬浮**：抬升 `translateY(-2px)`，底座拉长到 4px。
