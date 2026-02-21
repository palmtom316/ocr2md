import { useMemo, useState } from "react";

type LocalProfile = {
  name: string;
  provider: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  enabled: boolean;
};

type ProviderProfilePayload = {
  name: string;
  provider: string;
  base_url: string;
  api_key: string;
  model: string;
  enabled: boolean;
};

type StatusTone = "info" | "success" | "error";

declare global {
  interface Window {
    __TAURI__?: {
      core?: {
        invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
      };
    };
  }
}

const providerChoices = [
  { value: "openai", label: "OpenAI" },
  { value: "claude", label: "Claude" },
  { value: "gemini", label: "Gemini" },
  { value: "relay", label: "OpenAI Relay" },
  { value: "cc-switch", label: "CC-Switch Relay" }
];

const initialProfiles: LocalProfile[] = [
  {
    name: "Primary OpenAI",
    provider: "openai",
    baseUrl: "https://api.openai.com/v1",
    apiKey: "",
    model: "gpt-4.1-mini",
    enabled: true
  }
];

const queueSample = [
  { name: "annual-report.pdf", status: "Running OCR", tone: "is-running" },
  { name: "invoice-batch.docx", status: "Queued", tone: "is-queued" },
  { name: "scan_2026_02_21.pdf", status: "Retrying", tone: "is-warning" },
  { name: "contract-dual-layer.pdf", status: "Completed", tone: "is-success" }
];

function toLocalProfile(payload: ProviderProfilePayload): LocalProfile {
  return {
    name: payload.name,
    provider: payload.provider,
    baseUrl: payload.base_url,
    apiKey: payload.api_key,
    model: payload.model,
    enabled: payload.enabled
  };
}

function toPayload(profile: LocalProfile): ProviderProfilePayload {
  return {
    name: profile.name,
    provider: profile.provider,
    base_url: profile.baseUrl,
    api_key: profile.apiKey,
    model: profile.model,
    enabled: profile.enabled
  };
}

async function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) {
    throw new Error("Tauri runtime is unavailable in browser preview mode.");
  }
  return invoke<T>(command, args);
}

type IconProps = {
  className?: string;
  label?: string;
};

function QueueIcon({ className, label }: IconProps) {
  return (
    <svg
      aria-label={label}
      viewBox="0 0 24 24"
      role="img"
      className={className}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="4" y="4" width="16" height="16" rx="3" />
      <path d="M8 9h8M8 13h8M8 17h5" />
    </svg>
  );
}

function WorkspaceIcon({ className, label }: IconProps) {
  return (
    <svg
      aria-label={label}
      viewBox="0 0 24 24"
      role="img"
      className={className}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M4 5h7v7H4zM13 5h7v4h-7zM13 11h7v8h-7zM4 14h7v5H4z" />
    </svg>
  );
}

function SettingsIcon({ className, label }: IconProps) {
  return (
    <svg
      aria-label={label}
      viewBox="0 0 24 24"
      role="img"
      className={className}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M12 3v4M12 17v4M3 12h4M17 12h4" />
      <circle cx="12" cy="12" r="4.3" />
    </svg>
  );
}

function StatusDot({ className }: { className: string }) {
  return (
    <svg
      viewBox="0 0 16 16"
      role="img"
      aria-hidden="true"
      className={className}
      fill="currentColor"
    >
      <circle cx="8" cy="8" r="5.5" />
    </svg>
  );
}

export function LayoutShell() {
  const [passphrase, setPassphrase] = useState("");
  const [profiles, setProfiles] = useState<LocalProfile[]>(initialProfiles);
  const [activeProfileIndex, setActiveProfileIndex] = useState(0);
  const [isWorking, setIsWorking] = useState(false);
  const [statusTone, setStatusTone] = useState<StatusTone>("info");
  const [statusMessage, setStatusMessage] = useState(
    "Configure provider credentials, then save encrypted profiles."
  );

  const activeProfile = profiles[activeProfileIndex] ?? profiles[0];
  const safeProfile = useMemo<LocalProfile>(
    () =>
      activeProfile ?? {
        name: "",
        provider: "openai",
        baseUrl: "",
        apiKey: "",
        model: "",
        enabled: true
      },
    [activeProfile]
  );

  function setProfileField<K extends keyof LocalProfile>(key: K, value: LocalProfile[K]) {
    setProfiles((previous) =>
      previous.map((item, index) => (index === activeProfileIndex ? { ...item, [key]: value } : item))
    );
  }

  function addProfile() {
    setProfiles((previous) => [
      ...previous,
      {
        name: `Profile ${previous.length + 1}`,
        provider: "openai",
        baseUrl: "https://api.openai.com/v1",
        apiKey: "",
        model: "gpt-4.1-mini",
        enabled: true
      }
    ]);
    setActiveProfileIndex(profiles.length);
  }

  function removeProfile() {
    if (profiles.length <= 1) {
      return;
    }
    setProfiles((previous) => previous.filter((_, index) => index !== activeProfileIndex));
    setActiveProfileIndex((previous) => Math.max(0, previous - 1));
  }

  async function handleLoadProfiles() {
    setIsWorking(true);
    try {
      const loaded = await invokeTauri<ProviderProfilePayload[]>("load_profiles", { passphrase });
      if (loaded.length === 0) {
        setStatusTone("info");
        setStatusMessage("No encrypted profiles found. Save one to create your settings vault.");
      } else {
        setProfiles(loaded.map(toLocalProfile));
        setActiveProfileIndex(0);
        setStatusTone("success");
        setStatusMessage(`Loaded ${loaded.length} encrypted profile(s).`);
      }
    } catch (error) {
      setStatusTone("error");
      setStatusMessage(
        error instanceof Error ? error.message : "Failed to load profiles from encrypted store."
      );
    } finally {
      setIsWorking(false);
    }
  }

  async function handleSaveProfiles() {
    setIsWorking(true);
    try {
      await invokeTauri("save_profiles", {
        passphrase,
        profiles: profiles.map(toPayload)
      });
      setStatusTone("success");
      setStatusMessage(`Saved ${profiles.length} profile(s) to encrypted storage.`);
    } catch (error) {
      setStatusTone("error");
      setStatusMessage(
        error instanceof Error ? error.message : "Failed to save profiles into encrypted store."
      );
    } finally {
      setIsWorking(false);
    }
  }

  return (
    <main className="workbench">
      <header className="topbar">
        <div className="brand">
          <img src="/favicon.png" alt="Ocr2Md app icon" className="brand-icon" />
          <div>
            <h1>Ocr2Md Workbench</h1>
            <p>Colorblind-safe OCR to Markdown orchestration for macOS and Windows.</p>
          </div>
        </div>
        <div className="meta-chip">
          <StatusDot className="dot is-running" />
          <span>Queue Online</span>
        </div>
      </header>

      <div className="shell">
        <aside className="queue-pane" data-testid="queue-pane">
          <section className="pane-head">
            <QueueIcon className="pane-icon" label="Queue navigation icon" />
            <div>
              <h2>Queue Center</h2>
              <p>Batch input and job routing</p>
            </div>
          </section>

          <nav className="nav-list" aria-label="Navigation">
            <button className="nav-item is-active" type="button">
              <QueueIcon className="nav-icon" />
              <span>Queue</span>
            </button>
            <button className="nav-item" type="button">
              <WorkspaceIcon className="nav-icon" />
              <span>Markdown</span>
            </button>
            <button className="nav-item" type="button">
              <SettingsIcon className="nav-icon" />
              <span>Settings</span>
            </button>
          </nav>

          <ul className="job-list">
            {queueSample.map((job) => (
              <li key={job.name} className="job-card">
                <div>
                  <strong>{job.name}</strong>
                  <p>Provider route auto-selected by profile policy</p>
                </div>
                <span className={`status-pill ${job.tone}`}>
                  <StatusDot className={`dot ${job.tone}`} />
                  {job.status}
                </span>
              </li>
            ))}
          </ul>
        </aside>

        <section className="workspace-pane" data-testid="workspace-pane">
          <div className="workspace-grid">
            <article className="workspace-main">
              <header className="pane-head">
                <WorkspaceIcon className="pane-icon" label="Workspace pane icon" />
                <div>
                  <h2>Workspace</h2>
                  <p>Markdown, OCR source, and diagnostics</p>
                </div>
              </header>

              <div className="tab-row">
                <button type="button" className="tab is-active">
                  Markdown
                </button>
                <button type="button" className="tab">
                  OCR Text
                </button>
                <button type="button" className="tab">
                  Logs
                </button>
              </div>

              <div className="preview">
                <pre>{`# Document Title

## Summary
- Structured markdown output appears here.
- Diff and logs stay available on adjacent tabs.

## Next
1. Verify OCR output quality
2. Tune provider/model for formatting
3. Export .md`}</pre>
              </div>
            </article>

            <aside className="settings-pane">
              <header className="pane-head compact">
                <SettingsIcon className="pane-icon" label="Settings pane icon" />
                <div>
                  <h2>API Settings</h2>
                  <p>Encrypted profile storage</p>
                </div>
              </header>

              <label className="field">
                <span>Master passphrase</span>
                <input
                  aria-label="Master passphrase"
                  type="password"
                  value={passphrase}
                  onChange={(event) => setPassphrase(event.target.value)}
                  placeholder="Enter encryption passphrase"
                />
              </label>

              <div className="row">
                <label className="field">
                  <span>Profile</span>
                  <select
                    value={String(activeProfileIndex)}
                    onChange={(event) => setActiveProfileIndex(Number(event.target.value))}
                  >
                    {profiles.map((profile, index) => (
                      <option value={index} key={`${profile.name}-${index}`}>
                        {profile.name}
                      </option>
                    ))}
                  </select>
                </label>
                <div className="stack-actions">
                  <button type="button" onClick={addProfile}>
                    Add
                  </button>
                  <button type="button" onClick={removeProfile}>
                    Remove
                  </button>
                </div>
              </div>

              <label className="field">
                <span>Profile name</span>
                <input
                  value={safeProfile.name}
                  onChange={(event) => setProfileField("name", event.target.value)}
                />
              </label>

              <label className="field">
                <span>Provider</span>
                <select
                  aria-label="Provider"
                  value={safeProfile.provider}
                  onChange={(event) => setProfileField("provider", event.target.value)}
                >
                  {providerChoices.map((provider) => (
                    <option value={provider.value} key={provider.value}>
                      {provider.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field">
                <span>Base URL</span>
                <input
                  aria-label="Base URL"
                  value={safeProfile.baseUrl}
                  onChange={(event) => setProfileField("baseUrl", event.target.value)}
                />
              </label>

              <label className="field">
                <span>API Key</span>
                <input
                  aria-label="API Key"
                  type="password"
                  value={safeProfile.apiKey}
                  onChange={(event) => setProfileField("apiKey", event.target.value)}
                />
              </label>

              <label className="field">
                <span>Model</span>
                <input
                  aria-label="Model"
                  value={safeProfile.model}
                  onChange={(event) => setProfileField("model", event.target.value)}
                />
              </label>

              <label className="switch-field">
                <input
                  type="checkbox"
                  checked={safeProfile.enabled}
                  onChange={(event) => setProfileField("enabled", event.target.checked)}
                />
                <span>Enabled for routing</span>
              </label>

              <div className="action-row">
                <button type="button" onClick={handleLoadProfiles} disabled={isWorking}>
                  Load Profiles
                </button>
                <button type="button" onClick={handleSaveProfiles} disabled={isWorking}>
                  Save Profiles
                </button>
              </div>

              <p className={`status-line ${statusTone}`}>{statusMessage}</p>
            </aside>
          </div>
        </section>
      </div>
    </main>
  );
}
