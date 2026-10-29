import type { Condizione } from "../pdf/generatePdf";

export const MEMORIA_OPTIONS: { value: string; label: string }[] = [
  { value: "64", label: "64 GB" },
  { value: "128", label: "128 GB" },
  { value: "256", label: "256 GB" },
  { value: "512", label: "512 GB" },
  { value: "1024", label: "1 TB" },
  { value: "2048", label: "2 TB" },
];

export interface TagDraft {
  id: string;
  nome: string;
  gb: string;
  prezzo: string;
  prezzoOriginale: string;
  inOfferta: boolean;
  condizione: Condizione;
}

interface TagCardProps {
  draft: TagDraft;
  index: number;
  onChange: (patch: Partial<TagDraft>) => void;
  onRemove: () => void;
}

export default function TagCard({ draft, index, onChange, onRemove }: TagCardProps) {
  return (
    <div className={`tag-card${draft.inOfferta ? " offerta" : ""}`}>
      <span className="tag-num">{index + 1}</span>

      <div className="tag-fields">
        <label className="field nome">
          <span>Nome dispositivo</span>
          <input
            type="text"
            value={draft.nome}
            placeholder="es. Galaxy A17"
            onChange={(e) => onChange({ nome: e.target.value })}
          />
        </label>

        <label className="field gb">
          <span>Memoria</span>
          <select value={draft.gb} onChange={(e) => onChange({ gb: e.target.value })}>
            {MEMORIA_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </label>

        <label className="field prezzo">
          <span>Prezzo (€)</span>
          <input
            type="number"
            min={0}
            step={0.01}
            value={draft.prezzo}
            placeholder="es. 199"
            onChange={(e) => onChange({ prezzo: e.target.value })}
          />
        </label>

        <label className="field condizione">
          <span>Condizione</span>
          <select
            value={draft.condizione}
            onChange={(e) => onChange({ condizione: e.target.value as Condizione })}
          >
            <option value="ricondizionato">Ricondizionato</option>
            <option value="nuovo">Nuovo</option>
          </select>
        </label>

        <label className="field offerta-check">
          <input
            type="checkbox"
            checked={draft.inOfferta}
            onChange={(e) => onChange({ inOfferta: e.target.checked })}
          />
          <span>In offerta</span>
        </label>

        {draft.inOfferta && (
          <label className="field prezzo-orig">
            <span>Prezzo originale (€)</span>
            <input
              type="number"
              min={0}
              step={0.01}
              value={draft.prezzoOriginale}
              placeholder="facoltativo"
              onChange={(e) => onChange({ prezzoOriginale: e.target.value })}
            />
          </label>
        )}
      </div>

      <button type="button" className="remove-btn" title="Rimuovi cartellino" onClick={onRemove}>
        ×
      </button>
    </div>
  );
}
