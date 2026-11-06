import { invoke } from "@tauri-apps/api/core";

export function registra(evento: string, dettaglio = "") {
  const riga = (dettaglio ? `${evento} ${dettaglio}` : evento).slice(0, 400);
  void invoke("diagnostica_log", { riga }).catch(() => undefined);
}
