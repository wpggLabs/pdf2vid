import type { ProviderOption } from "../types";

interface ProviderFieldProps {
  title: string;
  value: string;
  options: ProviderOption[];
  onChange: (value: string) => void;
}

export function ProviderField({ title, value, options, onChange }: ProviderFieldProps) {
  return (
    <label>
      {title}
      <select value={value} onChange={(event) => onChange(event.target.value)}>
        {options.map((provider) => (
          <option
            key={`${provider.category}-${provider.id}`}
            value={provider.id}
            disabled={!provider.implemented}
          >
            {provider.label} · {provider.kind === "local" ? "Free" : "BYO key"}
            {!provider.implemented ? " · Coming soon" : ""}
          </option>
        ))}
      </select>
    </label>
  );
}