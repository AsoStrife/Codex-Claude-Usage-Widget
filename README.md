# 🤖 AI Usage Widget

<div align="center">

**Widget elegante per Windows 11** che monitora il consumo di quota di **Codex** (OpenAI) e **Claude Code** (Anthropic) direttamente dalla system tray.

[![Build](https://img.shields.io/badge/Build-Passing-brightgreen)](#) 
[![Windows 11](https://img.shields.io/badge/Windows-11+-0078D4)](#) 
[![Vue 3](https://img.shields.io/badge/Vue-3-4FC08D)](#) 
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6)](#) 
[![Tauri 2](https://img.shields.io/badge/Tauri-2-FFC131)](#)

[📖 Documentazione](#-architettura) • [🚀 Quick Start](#-installation) • [🛠️ Build](#-come-buildare) • [🔐 Privacy](#-privacy)

</div>

---

## 📸 Anteprima

Cliccando l'icona nella system tray, appare un'elegante finestra popup:

```
┌──────────────────────────────────────┐
│ AI Usage                        🔄 ⚙ │
│                                      │
│ ● CODEX   PLUS                 11s ago│
│ 5h    [████████████░░░░]  98% left   │
│       resets in 4h 37m               │
│ Weekly [██████████░░░░░░]  71% left  │
│       Saturday 11:00                 │
│                                      │
│ ● CLAUDE  PRO                  12m ago│
│ 5h    [██████████████░░░░]  90% left │
│ Weekly [──────────────────]  Unavail.│
│                                      │
│ Updated 11s ago          Esc to close│
└──────────────────────────────────────┘
```

📍 **Niente taskbar, niente finestre normali** — Appare solo nella tray!

---

## ✨ Funzionalità Principali

### 🎯 Core
- **📊 Monitoraggio Live** — Visualizza quota 5-hour e weekly per Codex e Claude
- **🔄 Auto-Refresh** — Aggiornamenti automatici + refresh on-demand
- **⏱️ Countdown Precisi** — Mostra esattamente quando le quote si resettano
- **🪟 Windows 11 Native** — Comportamento nativo della system tray (no taskbar window)
- **⚡ Leggero** — Bassissimo footprint di memoria e CPU
- **🎨 Dark Theme** — UI moderna e ottimizzata

### 🔌 Integrazioni Provider

#### **Codex (OpenAI)**
✅ Autodetection CLI automatico  
✅ JSON-RPC via App Server (nessun accesso file auth)  
✅ Supporto multi-window (5h, weekly, custom)  
✅ Real-time updates + fallback polling  

#### **Claude Code (Anthropic)**
✅ Bridge locale per status-line integration  
✅ Nessuna credenziale richiesta  
✅ Cache intelligente con filesystem watcher  
✅ Preservazione configurazione status-line esistente  

### ⚙️ Impostazioni & Controlli
- **Avvio Automatico** — Parte con Windows
- **Integrazione Claude** — Abilita con un clic
- **Diagnostica** — Ispeziona versioni, percorsi, stati
- **Comandi Rapidi**:
  - `Esc` → Chiudi popup
  - `Ctrl+R` → Refresh manuale
  - Click tray → Toggle popup
  - Right-click tray → Menu nativo

---

## 🚀 Installation & Quick Start

### Prerequisiti

| Requisito | Versione |
|-----------|----------|
| **Windows** | 11 (x64) |
| **Node.js** | ≥ 20 LTS |
| **npm** | ≥ 9 |
| **Rust** | ≥ 1.77 (solo per build, opzionale per dev) |
| **WebView2** | Auto-installato dall'installer |

### Codex e Claude (opzionali)
- Se Codex è installato, il widget lo rileva automaticamente
- Se Claude Code è installato, il widget lo rileva automaticamente
- Nessuno è obbligatorio — il widget mostra "Not installed" se assente

### Dev Mode (Hot Reload)

```bash
# Clone il repo
git clone https://github.com/AsoStrife/Codex-Code-Widget.git
cd Codex-Code-Widget

# Installa dipendenze
npm install

# Avvia in development (Vite + Tauri, hot reload)
npm run dev              # Solo browser
npm start                # App desktop (Tauri)
```

---

## 🛠️ Come Buildare

### Build Frontend Solo
```bash
npm run build
npm run typecheck
```

### Build Desktop App Completo

```bash
# Genera icone
npm run icons

# Build completo (tutti i formati)
npm run build:exe

# Oppure singolarmente:
npm run build:portable   # Eseguibile standalone
npm run build:nsis       # Installer NSIS
npm run build:msi        # Installer MSI
```

**Output in `./builds/`:**
```
ai-usage-widget.exe                    Portable executable
AI Usage Widget_x.y.z_x64-setup.exe    NSIS installer  
AI Usage Widget_x.y.z_x64_en-US.msi    MSI installer
```

### Build Backend Rust

```bash
npm run typecheck        # Verifica tipi Rust
npm run test:rust        # Test unitari Rust
npm run lint:rust        # Clippy linter
npm run fmt:rust         # Format code
```

### Utilità

```bash
npm test                 # Frontend tests (Vitest)
npm run test:watch       # Watch mode
npm run test:all         # Test JS + Rust
npm run clean            # Pulisci build artifacts
```

---

## 🔌 Come Funzionano le Integrazioni

### Codex (OpenAI)

Il widget mantiene un processo `codex app-server` live e comunica via **JSON-RPC over stdio**:

```
1. Widget avvia: codex app-server
2. Invia: { "method": "initialize" }
3. Riceve: response + capabilities
4. Chiama: { "method": "account/rateLimits/read" }
5. Ascolta: account/rateLimits/updated notifications
```

**Dettagli:**
- Le finestre sono matchate per durata (`windowDurationMins`): 300 = 5h, 10080 = weekly
- Restart automatico con backoff se il processo termina
- Job object garantisce terminazione pulita anche su crash
- Nessun accesso a `auth.json` — tutto tramite App Server

### Claude Code (Anthropic)

Claude Code delega i `rate_limits` al comando configurato come "status-line".

**Setup:**
1. Abilita "Claude Integration" nel widget (⚙ → Settings)
2. Widget si registra come:
   ```
   "C:\...\ai-usage-widget.exe" --claude-statusline-bridge
   ```

**Funzionamento:**
- Bridge mode legge JSON stdin (una volta), estrae solo quota numbers
- Scrive snapshot atomico in `%LOCALAPPDATA%\ai-usage-widget\claude-usage.json`
- Tauri/WebView **mai avviati** in questa modalità
- Widget principale watch il file → UI aggiorna in ~300ms

**Preservazione Status-line:**
- Se avevi una status-line custom, viene salvata e ripristinata
- Il bridge l'esegue ad ogni update e ne specchia l'output
- Disabilitando l'integrazione, la tua config torna al precedente

**First Run:** Claude mostra *Waiting for data* finché non usi Claude Code almeno una volta.

---

## 📋 Stack Tecnologico

| Layer | Tech | Nota |
|-------|------|------|
| **Desktop Shell** | Tauri 2 + Rust | System tray, window management, provider lifecycle |
| **Frontend** | Vue 3 + TypeScript | UI components, provider-agnostic |
| **State Mgmt** | Pinia 3 | Centralized reactive state |
| **Styling** | Tailwind CSS 4 | Utility-first CSS |
| **Build** | Vite 7 | Lightning-fast bundler |
| **Testing** | Vitest + Cargo | Unit + integration tests |
| **Quality** | vue-tsc + Clippy | Type safety, linting |

---

## 📂 Struttura del Progetto

```
Codex-Code-Widget/
│
├── src/                          # Frontend Vue 3 + TypeScript
│   ├── App.vue                   # Root component
│   ├── main.ts                   # Entry point
│   ├── style.css                 # Global theme + Tailwind
│   │
│   ├── components/               # Vue components
│   │   ├── ProviderCard.vue      # Card per provider
│   │   ├── QuotaRow.vue          # Riga quota (5h/weekly)
│   │   ├── ProgressBar.vue       # Barra progresso visuale
│   │   ├── WidgetHeader.vue      # Header + pulsanti
│   │   ├── WidgetFooter.vue      # Footer timestamp
│   │   ├── ProviderStatus.vue    # Stati (waiting, error, etc)
│   │   └── SettingsPanel.vue     # Panel settings/diagnostics
│   │
│   ├── stores/
│   │   ├── usage.ts              # Pinia store quota providers
│   │   └── settings.ts           # Pinia store settings
│   │
│   ├── types/
│   │   └── usage.ts              # Domain types (ProviderQuota, etc)
│   │
│   └── lib/
│       ├── tauri.ts              # Tauri invoke/events wrapper
│       ├── format.ts             # Formattazione data/percentuali
│       └── mock.ts               # Mock data per dev
│
├── src-tauri/                    # Backend Rust
│   ├── src/
│   │   ├── main.rs               # Tauri main entry
│   │   ├── app.rs                # App init
│   │   ├── tray.rs               # System tray icon + menu
│   │   ├── window.rs             # Popup window management
│   │   ├── state.rs              # Shared in-memory state
│   │   ├── commands.rs           # Tauri IPC commands
│   │   │
│   │   ├── providers/            # Provider-agnostic layer
│   │   │   ├── mod.rs
│   │   │   ├── types.rs          # Normalized ProviderQuota
│   │   │   ├── codex.rs          # Codex provider supervisor
│   │   │   └── claude.rs         # Claude provider supervisor
│   │   │
│   │   ├── codex/                # Codex-specific
│   │   │   ├── process.rs        # App Server subprocess
│   │   │   ├── rpc.rs            # JSON-RPC protocol
│   │   │   └── protocol.rs       # Message types
│   │   │
│   │   ├── claude/               # Claude-specific
│   │   │   ├── bridge.rs         # Status-line bridge mode
│   │   │   ├── config.rs         # Settings management
│   │   │   └── cache.rs          # Cache watcher
│   │   │
│   │   └── platform/
│   │       └── windows.rs        # Windows-specific code
│   │
│   └── Cargo.toml
│
├── scripts/
│   ├── generate-icons.mjs        # Icon generation
│   └── build-release.mjs         # Release orchestration
│
├── vite.config.ts                # Vite configuration
├── tsconfig.json                 # TypeScript config
├── package.json                  # npm dependencies
├── plan.md                        # Implementation plan dettagliato
└── README.md                      # Questo file
```

**Principio Architetturale:**
Concetti provider-specifici (JSON-RPC, status-line payloads, `primary`/`secondary`) **restano nel Rust backend**. 
Vue layer vede solo `ProviderQuota` normalizzato e provider-agnostico.

---

## 🎮 Comportamento dell'App

### Navigazione & Interazione
- **Left-click tray** → Apri/chiudi popup (sopra icona)
- **Right-click tray** → Menu nativo (Refresh, Settings, Quit, etc)
- **Drag header** → Sposta widget (posizione salvata)
- **Esc** → Chiudi popup (o settings panel se aperto)
- **Ctrl+R** → Refresh manuale

### Finestra Popup
- **Dimensioni:** ~380×330-420px (adattive al contenuto)
- **Posizionamento:** Sopra tray icon, fallback intelligente (multi-monitor, scaling 100-200%)
- **Comportamento:** Scompare al focus loss, no taskbar, always-on-top
- **Close:** Chiudere popup ≠ quitare l'app (use tray → Quit)

### Dati Mancanti
- Missing window → Mostrato come `Not reported`, mai come full bar
- Percentuali sempre "remaining" = `100 − used`
- Provider not installed → Chiaramente indicato
- Provider not authenticated → Chiaramente indicato
- Waiting for data → Label di stato esplicito

### Perché NO Token Counts?

Codex e Claude non espongono "fixed token bucket". Consumo varia per modello/workload.
Entrambi espongono solo: **% used** + **reset timestamp**.

Convertire in "tokens left" sarebbe una **guess presentata come fatto** — il widget mostra solo dati certi.

---

## 🔐 Privacy & Sicurezza

### Garanzie

✅ **Zero network** — Nessuna chiamata esterna  
✅ **Zero analytics** — No tracciamento  
✅ **Zero credenziali** — Non memorizza token, passwords  
✅ **No auth.json** — Non legge file Codex auth  
✅ **No OAuth tokens** — Non tocca Anthropic tokens  
✅ **No conversation content** — Non persiste chat  
✅ **Atomic cache** — Transazioni filesystem safe  

### Come Restano i Dati

**Codex:**
- Comunica solo via JSON-RPC protocol
- Zero accesso diretto al filesystem auth

**Claude:**
- Cache locale contiene SOLO:
  ```json
  {
    "fiveHour": {"usedPercentage": 23.5, "resetsAt": 1789502751},
    "weekly": {"usedPercentage": 41.2, "resetsAt": 1789808435},
    "updatedAt": 1789486202
  }
  ```
- Niente token, niente credenziali, niente payload Claude

**Memorizzazione:**
- Locale: `%LOCALAPPDATA%\ai-usage-widget\`
  - `settings.json` — autostart, integrazione toggle, etc
  - `claude-usage.json` — Claude cache snapshot
- Diagnostica mostra percorsi redatti

---

## 📊 Colori & Tema

Dark theme ottimizzato per la system tray:

| Elemento | Colore | Hex |
|----------|--------|-----|
| Panel Background | Dark | `#0e1117` |
| Panel Raised | Elevated | `#161b23` |
| Text | Ink | `#e8edf5` |
| Text Dim | Ink Dim | `#96a2b4` |
| Text Faint | Ink Faint | `#5d6877` |
| Border | Hairline | `#262d38` |
| Accent Codex | Verde | `#4ade80` |
| Accent Claude | Arancio | `#f59e6a` |
| Warn (Stale) | Giallo | `#fbbf24` |
| Error | Rosso | `#f87171` |

---

## 🧪 Testing

```bash
# Frontend unit tests
npm test
npm run test:watch

# Backend Rust tests
npm run test:rust

# Both
npm run test:all
```

**Acceptance Testing** (Windows 11):
- Scaling 100%, 125%, 150%, 200%
- Multi-monitor setups
- Taskbar posizioni varie
- Codex/Claude installed/missing
- Auth stato vario
- Focus loss handling
- Tray click toggling

---

## 🚀 Roadmap & Non-Goals

### Potenziali Migliorie
- 🌙 Light theme support
- 📊 Historical usage charts
- 🔔 Low quota notifications  
- 💾 Usage data export
- 🌍 macOS/Linux support
- 🎯 Per-provider settings

### Non-Goals MVP
- Credential management UI
- API billing dashboards
- Per-project analytics
- Cloud sync/accounts
- Browser integrations
- Token counting estimates

---

## 🐛 Troubleshooting

### Widget non appare in system tray
```bash
npm run clean
npm run icons
npm run build:exe
```

### Codex quota non si aggiorna
- Verifica: `codex app-server --help`
- Controlla Diagnostica (⚙ Settings)
- Rebuild: `npm run build:exe`

### Claude quota non funziona
- Verifica Claude Code autenticato
- Abilita integrazione (⚙ → Claude Integration)
- Usa Claude Code almeno una volta
- Controlla diagnostica

### High CPU / Performance
```bash
npm run clean
npm run build:exe
```

---

## 📝 Licenza

MIT — vedi [LICENSE](LICENSE)

---

## 🤝 Contributing

Bug found? 🐛 [Apri una issue](https://github.com/AsoStrife/Codex-Code-Widget/issues)!

Pull request benvenute! 🎉

```bash
# Setup dev environment
git clone https://github.com/AsoStrife/Codex-Code-Widget.git
cd Codex-Code-Widget
npm install
npm start
```

---

<div align="center">

**Made with ❤️ by Andrea Corriga**

Implements detailed [plan.md](plan.md) architecture.

[🌐 Website](https://visioscientiae.com) • [💻 GitHub](https://github.com/AsoStrife)

</div>
