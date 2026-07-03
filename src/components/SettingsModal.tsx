import { Gear, Key, X } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import type { ProviderList } from "../api";
import { getProviderList, storeApiKey } from "../backend";
import type { ProviderCategory, ProviderOption } from "../types";

interface Props {
  onClose: () => void;
  onOpenModels: () => void;
}

export function SettingsModal({ onClose, onOpenModels }: Props) {
  const [providers, setProviders] = useState<ProviderList | null>(null);
  const [category, setCategory] = useState<ProviderCategory>("voice");
  const [providerId, setProviderId] = useState<string>("");
  const [secret, setSecret] = useState("");
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    getProviderList().then((list) => {
      setProviders(list);
      if (list.voice.length > 0) setProviderId(list.voice[0].id);
    });
  }, []);

  async function handleSave() {
    if (!providerId || !secret.trim()) {
      setStatus("Pick a provider and enter a key");
      return;
    }
    try {
      await storeApiKey(providerId, secret.trim());
      setStatus(`${providerId} key saved in your system credential store`);
      setSecret("");
    } catch (e) {
      setStatus(`Could not save key: ${e}`);
    }
  }

  const options: ProviderOption[] = providers ? providers[category] : [];

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <section
        className="modal"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Provider settings"
      >
        <header>
          <h2>Provider settings</h2>
          <button type="button" className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </header>

        <div className="settings-tabs">
          {(["translation", "voice", "visual"] as ProviderCategory[]).map((cat) => (
            <button
              type="button"
              key={cat}
              className={cat === category ? "active" : ""}
              onClick={() => {
                setCategory(cat);
                if (providers && providers[cat].length > 0) {
                  setProviderId(providers[cat][0].id);
                }
              }}
            >
              {cat}
            </button>
          ))}
        </div>

        <label>
          Provider
          <select value={providerId} onChange={(event) => setProviderId(event.target.value)}>
            {options.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.label} · {provider.kind === "local" ? "Free" : "BYO key"}
                {!provider.implemented ? " · Coming soon" : ""}
              </option>
            ))}
          </select>
        </label>

        <label>
          API key
          <input
            type="password"
            value={secret}
            onChange={(event) => setSecret(event.target.value)}
            placeholder={options.find((o) => o.id === providerId)?.keyLabel ?? "No key required"}
            disabled={!options.find((o) => o.id === providerId)?.keyLabel}
          />
        </label>

        <p className="modal-note">
          <Key size={17} />
          Keys are stored in your operating system credential manager, never in project files.
        </p>

        {status && <p className="modal-status">{status}</p>}

        <button type="button" className="export-primary" onClick={handleSave}>
          Save securely
        </button>

        <button type="button" className="export-secondary" onClick={onOpenModels}>
          <Gear size={16} /> Manage local models
        </button>
      </section>
    </div>
  );
}
