# 乖乖Claude 🪔

> 用三炷香拜拜、或用愛的小手疼愛，催 Claude Code 加速。觸發後會自動貼上 `/btw ...` 催促語到 Claude Code 上。

基於 [badclaude](https://github.com/GitFrog1111/badclaude) 的精神，把鞭子換成三炷香 / 愛的小手，Electron 換成 Tauri (Rust)。

## 功能

- 🪔 常駐在 macOS 螢幕最上方的選單列（menu bar）／ Windows 右下角的工作列通知區，點擊圖示開關 overlay
- 🪔 圖示右鍵選單切換「三炷香模式」與「愛的小手模式」
- 🪔 **三炷香模式**：三柱香跟隨滑鼠透視晃動，上下移動滑鼠三次觸發拜拜，木魚音效 + 浮動文字 + 功德 +1
- ✋ **愛的小手模式**：小手跟隨滑鼠，左右快速揮動觸發巴掌，打擊音效 + 震動效果 + 巴掌數 +1
- 🪔 觸發後自動 `Cmd+Tab`（Mac）/ `Alt+Tab`（Win）切回前一個視窗，貼上隨機 `/btw ...` 催促語並按 Enter
- 🪔 多螢幕感知，overlay 永遠開在游標所在的螢幕
- 🪔 點擊 overlay 任意處即可隱藏

## 下載

到 [Releases 頁面](https://github.com/xavierforge/guaiguai-claude/releases) 抓對應平台的檔案:

| 平台 | 檔案 |
|------|------|
| macOS (Apple Silicon) | `guaiguai-claude-macOS-ARM.dmg` |
| macOS (Intel) | `guaiguai-claude-macOS-Intel.dmg` |
| Windows | `guaiguai-claude-Windows-x64.msi`(或 `...-setup.exe`) |

### macOS 首次開啟

本專案沒加入 Apple Developer Program,`.app` 沒有 notarize。首次打開時 Gatekeeper 會跳「『GuaiguaiClaude』已損毀，無法打開」，這不是真的壞掉,而是 Gatekeeper 對未 notarize + 下載附加 quarantine 屬性的誤導訊息。在終端機執行一次:

```bash
xattr -cr /Applications/GuaiguaiClaude.app
```

之後就能正常開啟。

接著到「系統設定 → 隱私權與安全性 → 輔助使用」把本 app 打勾(鍵盤模擬需要)。第一次觸發時還會跳出自動化權限提示,要求允許控制「System Events」,也按允許。觸發催促時會短暫使用系統剪貼簿(AppleScript 寫入 → Cmd+V 貼上)。

### Windows 首次開啟

`.msi` 未簽署,SmartScreen 會擋一次，點「更多資訊 → 仍要執行」即可。

## 從原始碼 build

本專案前端只有一份純靜態 `ui/index.html`,**完全不使用 npm / Node.js 生態系**。這是刻意的選擇，最近 npm 頻繁爆出供應鏈投毒事件(惡意套件、被劫持的 transitive dependency 等),一個幾百行的小玩具沒有理由去背負幾千個 npm 依賴的風險。整個工具鏈只需要 Rust + Tauri CLI。

### 前置需求

1. **Rust** — 透過 [rustup](https://rustup.rs/) 安裝(macOS/Linux 用 `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`;Windows 下載 `rustup-init.exe`)。Rust 1.70 以上皆可。
2. **Tauri CLI v2** — `cargo install tauri-cli --version "^2" --locked`
3. **平台工具鏈**:
   - macOS:Xcode Command Line Tools(`xcode-select --install`)
   - Windows:Visual Studio Build Tools 或完整 VS(C++ build tools)
   - 其他系統依賴可參考 [Tauri 官方 prerequisites](https://v2.tauri.app/start/prerequisites/)

### 開發模式(即時熱重載)

```bash
cd guaiguai-claude
cargo tauri dev
```

### 打包成安裝檔

```bash
cd guaiguai-claude
cargo tauri build
```

產出路徑:
- macOS: `src-tauri/target/release/bundle/dmg/*.dmg`(也可以直接跑 `src-tauri/target/release/bundle/macos/*.app`)
- Windows: `src-tauri/target/release/bundle/msi/*.msi`、`src-tauri/target/release/bundle/nsis/*.exe`

## 專案結構

```
guaiguai-claude/
├── ui/
│   └── index.html           # 兩個模式的 canvas、動畫、觸發偵測、音效
├── src-tauri/
│   ├── Cargo.toml           # Rust 依賴
│   ├── tauri.conf.json      # Tauri 設定（透明視窗、tray）
│   ├── capabilities/        # Tauri v2 權限
│   ├── icons/               # Tray icon、bundle icons
│   └── src/
│       ├── main.rs          # Tray 選單、overlay 控制、trigger_action 指令
│       └── macro_sender.rs  # 跨平台鍵盤/剪貼簿注入
└── README.md
```

## 自訂

催促語清單在 `src-tauri/src/main.rs` 的 `INCENSE_PHRASES` 和 `SLAPPER_PHRASES` 常數裡。

`ui/index.html` 頂部的 `C` 物件可調整視覺與互動：

| 參數 | 說明 |
|------|------|
| `stickLength` / `stickWidth` | 香的長度與粗細 |
| `tiltFactor` / `tiltMax` | 香前後晃動靈敏度與幅度 |
| `swayFactor` / `swayMax` | 香左右晃動靈敏度與幅度 |
| `bowAmplitude` / `bowsNeeded` | 觸發拜拜所需的移動幅度與次數 |
| `incenseMessages` / `slapMessages` | 兩個模式的浮動文字清單 |
| `incenseTriggerChance` / `slapTriggerChance` | 兩個模式觸發時實際送出催促語的機率 |
| `slapAngVelThreshold` | 愛的小手觸發所需的揮擊角速度門檻 |
| `smokePerFrame` | 三炷香煙霧濃度 |

## 致敬

- [badclaude](https://github.com/GitFrog1111/badclaude) — 原版鞭子概念
- 南無加速菩薩 🙏
- 爸媽的愛的小手 ✋
