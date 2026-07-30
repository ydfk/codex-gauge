# 使用 Xcode 在本机安装与更新

本文适用于只在自己的 Mac 上使用 Codex Gauge、不制作公开安装包的场景。

推荐将 Xcode 用于开发和验证，再把 Release 版 `Codex Gauge.app` 固定安装到 `/Applications`。不需要制作 DMG 或 PKG，也不需要 Developer ID、公证或 Sparkle 自动更新。

## 1. 第一次配置 Xcode

在 `native-macos` 目录执行：

```bash
open CodexGaugeNative.xcodeproj
```

Xcode 打开后：

1. 在左侧项目导航中选择蓝色的 `CodexGaugeNative` 工程。
2. 在 Targets 中选择 `CodexGaugeNative`。
3. 打开 **Signing & Capabilities**。
4. 勾选 **Automatically manage signing**。
5. 在 **Team** 中选择自己的 Personal Team。
6. 在 Xcode 顶部将运行目标设为 **My Mac**。

默认 Bundle Identifier 是 `com.ydfk.codex-gauge.native`。只要 Xcode 没有报告冲突，就不要修改它；改变 Bundle Identifier 会让 macOS 将应用视为另一个程序，并可能影响登录启动注册。

> [!NOTE]
> Personal Team 和 Apple Development 证书足以在自己的 Mac 上运行。Developer ID Application 和 Apple 公证只用于向其他用户分发。

## 2. 开发和测试

在 Xcode 中按 `Command + R`，或者选择 **Product → Run**：

1. Xcode 编译 Debug 版本。
2. 应用启动后不会显示在 Dock。
3. 在 macOS 顶部菜单栏点击 Codex Gauge 图标查看界面。

按 `Command + B` 可以只检查编译，按 `Command + .` 可以停止当前运行。

命令行也可以构建未签名 Debug 版本：

```bash
./scripts/build.sh
```

产物位于：

```text
DerivedData/Build/Products/Debug/Codex Gauge.app
```

Debug 版适合验证代码，但不要长期从 `DerivedData` 运行。Xcode 清理缓存后这个路径可能失效，登录启动也可能继续指向已经被清理的应用。

## 3. 生成本机长期使用的 Release 版

确认 Debug 版工作正常后，在 Xcode 中选择 **Product → Archive**。

Archive 完成后会打开 Organizer：

1. 选择刚生成的 Codex Gauge Archive。
2. 点击 **Distribute App**。
3. 选择 **Copy App**；如果先出现 **Custom**，选择 **Custom → Copy App**。
4. 选择一个临时导出目录。
5. 导出完成后会得到 `Codex Gauge.app`。

本机自用不要选择 App Store Connect 或 Developer ID 分发。

## 4. 安装到 Applications

安装前先退出所有正在运行的 Codex Gauge，包括 Xcode 启动的 Debug 版，避免同时出现两个菜单栏图标。

把 Organizer 导出的 `Codex Gauge.app` 拖入：

```text
/Applications
```

最终路径应为：

```text
/Applications/Codex Gauge.app
```

可以在 Finder 中双击启动，也可以执行：

```bash
open "/Applications/Codex Gauge.app"
```

第一次从 `/Applications` 启动后，如果需要登录时自动运行，请在设置中先关闭再重新开启“登录时启动”。这样 macOS 会登记稳定的 `/Applications` 路径，而不是 Xcode 的临时构建路径。

## 5. 修改代码后的替换流程

以后每次更新按以下顺序操作：

1. 修改代码前同步远程仓库：

   ```bash
   git pull --rebase --autostash origin main
   ```

2. 用 Xcode 打开工程，按 `Command + R` 验证新功能。
3. 测试通过后选择 **Product → Archive**。
4. 在 Organizer 中选择 **Distribute App → Copy App**。
5. 从应用面板点击“退出”，关闭当前 `/Applications/Codex Gauge.app`。
6. 将新导出的 `Codex Gauge.app` 拖入 `/Applications`。
7. Finder 提示已有同名应用时选择“替换”。
8. 重新打开 `/Applications/Codex Gauge.app`。

不需要先卸载旧版本，也不要删除 Application Support 数据目录。

## 6. 版本号建议

在 Xcode 中选择 **Target → General**，可以看到：

- **Version**：对应 `MARKETING_VERSION`，例如 `0.1.0`。
- **Build**：对应 `CURRENT_PROJECT_VERSION`，例如 `1`。

只修改细节时可以递增 Build：

```text
Version 0.1.0
Build 1 → 2 → 3
```

功能版本发生变化时再调整 Version：

```text
0.1.0 → 0.2.0
```

本机覆盖安装不强制增加版本号，但递增 Build 可以帮助确认 `/Applications` 中运行的是不是最新构建。

## 7. 配置与数据保留

覆盖 `/Applications/Codex Gauge.app` 只替换应用程序，不会删除本地配置和用量快照。它们保存在：

```text
~/Library/Application Support/Codex Gauge Native/
```

只要不删除这个目录，菜单栏显示方式、刷新间隔、数据源设置和上一次成功快照都会保留。

## 8. 常见问题

### 菜单栏出现两个图标

通常是 Xcode Debug 版和 `/Applications` Release 版同时运行。先在两个面板中分别点击“退出”，然后只启动 `/Applications/Codex Gauge.app`。

### Finder 提示应用正在使用

先通过应用面板的“退出”按钮关闭旧版本，再执行替换。不要在进程仍运行时覆盖应用包。

### 登录后启动了旧版本

退出所有 Codex Gauge，只打开 `/Applications/Codex Gauge.app`，然后在设置中将“登录时启动”关闭再开启一次。

### 更新后设置不见了

确认 Bundle Identifier 没有被修改，并检查 `~/Library/Application Support/Codex Gauge Native/` 是否仍然存在。正常覆盖 `.app` 不会删除这个目录。

## 推荐工作流

```text
修改代码
  ↓
Command + R 测试
  ↓
Product → Archive
  ↓
Distribute App → Copy App
  ↓
退出旧版本
  ↓
覆盖 /Applications/Codex Gauge.app
  ↓
重新启动
```
