import amazon from "./assets/amazon.png";
import epol from "./assets/epol.jpg";
import bitrix24 from "./assets/bitrix24.png";
import type { FornitoreSalvato } from "./model";

const ICONE: Record<string, string> = { amazon, epol, bitrix24 };

export function conIconaFornitore(voce: FornitoreSalvato): FornitoreSalvato {
  if (voce.immagine) return voce;
  const icona = ICONE[voce.id];
  return icona ? { ...voce, immagine: icona } : voce;
}

export function senzaIconaPredefinita(voce: FornitoreSalvato): FornitoreSalvato {
  if (ICONE[voce.id] === voce.immagine) return { ...voce, immagine: "" };
  return voce;
}
