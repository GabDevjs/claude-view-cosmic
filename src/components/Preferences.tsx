import type { Preferences } from "../types";

interface Props {
  prefs: Preferences;
  onChange: (next: Preferences) => void;
  onAutostartChange: (enable: boolean) => void;
  onClose: () => void;
  error: string | null;
}

export function PreferencesPanel({
  prefs,
  onChange,
  onAutostartChange,
  onClose,
  error,
}: Props) {
  const patch = <K extends keyof Preferences>(
    section: K,
    update: Partial<Preferences[K]>,
  ) => {
    onChange({ ...prefs, [section]: { ...prefs[section], ...update } });
  };

  return (
    <div className="prefs-overlay" onClick={onClose}>
      <div className="prefs-panel" onClick={(e) => e.stopPropagation()}>
        <header>
          <h2>Preferences</h2>
          <button className="close" onClick={onClose} aria-label="close">
            ×
          </button>
        </header>

        <section>
          <h3>Display</h3>
          <label>
            <input
              type="checkbox"
              checked={prefs.display.show_percentage}
              onChange={(e) => patch("display", { show_percentage: e.target.checked })}
            />
            Show % next to tray icon
          </label>
          <label>
            Alert threshold
            <input
              type="number"
              min={0}
              max={100}
              value={prefs.display.alert_threshold}
              onChange={(e) =>
                patch("display", { alert_threshold: Number(e.target.value) })
              }
            />
            %
          </label>
          <label>
            History window
            <input
              type="number"
              min={1}
              max={90}
              value={prefs.display.history_window_days}
              onChange={(e) =>
                patch("display", { history_window_days: Number(e.target.value) })
              }
            />
            days
          </label>
          <label>
            Refresh interval
            <input
              type="number"
              min={5}
              max={3600}
              value={prefs.display.refresh_interval_seconds}
              onChange={(e) =>
                patch("display", {
                  refresh_interval_seconds: Number(e.target.value),
                })
              }
            />
            s
          </label>
        </section>

        <section>
          <h3>Behavior</h3>
          <label>
            <input
              type="checkbox"
              checked={prefs.behavior.autostart}
              onChange={(e) => onAutostartChange(e.target.checked)}
            />
            Launch on login
          </label>
          <label>
            <input
              type="checkbox"
              checked={prefs.behavior.notifications_enabled}
              onChange={(e) =>
                patch("behavior", { notifications_enabled: e.target.checked })
              }
            />
            Desktop notifications
          </label>
        </section>

        <section>
          <h3>Appearance</h3>
          <label>
            Theme
            <select
              value={prefs.appearance.theme}
              onChange={(e) => patch("appearance", { theme: e.target.value })}
            >
              <option value="auto">auto</option>
              <option value="light">light</option>
              <option value="dark">dark</option>
            </select>
          </label>
        </section>

        {error && <p className="error">prefs: {error}</p>}
      </div>
    </div>
  );
}
