import { describe, expect, it } from "vitest";
import {
  campoMancante,
  cifreWhatsapp,
  compilaMessaggio,
  contatori,
  esitoRicerca,
  etichettaStato,
  daCsv,
  euro,
  MESSAGGIO_DEFAULT,
  nomeAutomatico,
  ordineDaBozza,
  parseEuro,
  ordina,
  toCsv,
  upsert,
  urlDiApertura,
  urlWhatsapp,
  bozzaNuova,
  type Ordine,
} from "../../src/tabs/ordini/model";

function ordine(patch: Partial<Ordine> = {}): Ordine {
  return {
    id: "1",
    cliente: "Ada",
    telefono: "3331112233",
    dispositivo: "iPhone 13",
    colore: "nero",
    cover: false,
    tipoCover: "normale",
    nomeProdotto: "Pellicola",
    fornitore: "amazon",
    immagine: "",
    stato: "da_ordinare",
    consegnato: false,
    dataOrdine: "2026-11-04",
    costo: 8,
    prezzoTotale: 12.5,
    acconto: 0,
    note: "",
    link: "",
    ...patch,
  };
}

describe("ordini", () => {
  it("conta totali, da completare e pronti solo tra gli attivi", () => {
    const numeri = contatori([
      ordine(),
      ordine({ id: "2", stato: "pronto_ritiro" }),
      ordine({ id: "3", stato: "pronto_ritiro", consegnato: true }),
    ]);
    expect(numeri).toMatchObject({ totali: 3, daOrdinare: 1, ordinati: 0, pronti: 1, attivi: 2, consegnati: 1 });
  });

  it("ordina per costo e inverte il verso", () => {
    const lista = [ordine({ id: "a", costo: 2 }), ordine({ id: "b", costo: 10 })];
    expect(ordina(lista, "costo", "asc").map((voce) => voce.id)).toEqual(["a", "b"]);
    expect(ordina(lista, "costo", "desc").map((voce) => voce.id)).toEqual(["b", "a"]);
  });
  it("passa all'altra scheda solo se la corrente non ha risultati", () => {
    const ordini = [ordine({ consegnato: true })];
    expect(esitoRicerca(ordini, "attivi", "ada").passaA).toBe("consegnati");
    expect(esitoRicerca(ordini, "consegnati", "ada").passaA).toBeNull();
    expect(esitoRicerca(ordini, "attivi", "").passaA).toBeNull();
  });

  it("compone l'url WhatsApp con prefisso 39 e lascia un numero già prefissato", () => {
    expect(cifreWhatsapp("333 1112233")).toBe("393331112233");
    expect(cifreWhatsapp("+44 7911 123456")).toBe("447911123456");
    expect(urlWhatsapp("", "ciao")).toBeNull();
    const url = urlWhatsapp("3331112233", "Ciao Ada");
    expect(url).toBe("https://wa.me/393331112233?text=Ciao%20Ada");
  });

  it("inserisce un ordine nuovo e aggiorna lo stesso id senza toccare gli altri", () => {
    const primo = ordine();
    const secondo = ordine({ id: "2", cliente: "Nino" });
    const aggiornato = upsert([primo, secondo], { ...primo, cliente: "Ada Lovelace" });
    expect(aggiornato.map((voce) => voce.cliente)).toEqual(["Ada Lovelace", "Nino"]);
    expect(upsert([primo], secondo)).toHaveLength(2);
  });

  it("rifiuta il salvataggio senza cliente o modello e scrive il csv", () => {
    expect(campoMancante(bozzaNuova())).toBe("cliente");
    const senzaModello = { ...bozzaNuova(), cliente: "Ada" };
    expect(campoMancante(senzaModello)).toBe("dispositivo");
    expect(nomeAutomatico({ dispositivo: "Galaxy A17", cover: true, tipoCover: "libro" })).toBe(
      "Cover a libro per Galaxy A17",
    );
    expect(nomeAutomatico({ dispositivo: "Galaxy A17", cover: true, tipoCover: "normale" })).toBe(
      "Cover per Galaxy A17",
    );
    expect(nomeAutomatico({ dispositivo: "Galaxy A17", cover: false, tipoCover: "libro" })).toBe("Galaxy A17");
    const salvato = ordineDaBozza("9", false, { ...senzaModello, dispositivo: "Pixel", costo: "10,00", prezzo: "1.234,50" });
    expect(salvato.nomeProdotto).toBe("Pixel");
    expect(salvato.costo).toBe(10);
    expect(salvato.prezzoTotale).toBe(1234.5);
    expect(etichettaStato(salvato.stato)).toBe("Da ordinare");
    expect(euro(salvato.prezzoTotale)).toBe("1.234,50");
    expect(parseEuro("12.50")).toBe(12.5);
    const csv = toCsv([{ ...salvato, link: "https://doctorphone.bitrix24.it/crm/deal/1" }]);
    const importato = daCsv(csv, [{ id: "amazon", nome: "Amazon", immagine: "" }]).ordini;
    expect(importato).toHaveLength(1);
    expect(importato[0]?.link).toBe("https://doctorphone.bitrix24.it/crm/deal/1");
    expect(urlDiApertura("https://e-pol.bitrix24.it/company/personal/user/1/tasks/task/view/2/")).toBe(
      "bx://v3/open/https://e-pol.bitrix24.it/company/personal/user/1/tasks/task/view/2/",
    );
    expect(urlDiApertura("https://www.amazon.it/dp/123")).toBe("https://www.amazon.it/dp/123");
    expect(csv).toContain("1.234,50");
    expect(compilaMessaggio("Resto {totale - acconto}", { ...salvato, prezzoTotale: 10, acconto: 4 })).toBe("Resto 6,00");
    expect(compilaMessaggio(MESSAGGIO_DEFAULT, { ...salvato, cliente: "Ada", nomeProdotto: "Cover", colore: "Nero", acconto: 0, prezzoTotale: 12 })).toBe(
      "Ciao Ada, il tuo Cover, Nero è pronto per il ritiro.\nTotale 12,00 €.",
    );
  });
});
