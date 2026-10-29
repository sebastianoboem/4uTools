/* =============================================================================
 * Seleziona tutte le foto in Google Foto
 * =============================================================================
 *
 * COSA FA
 *   Scorre automaticamente Google Foto e clicca la checkbox di selezione di
 *   ogni foto, saltando la checkbox "Salva tutte le foto ..." (il pulsante di
 *   selezione di un'intera giornata/sezione). Gestisce il caricamento lazy:
 *   scrolla a step, attende il rendering e ripassa per non lasciare indietro
 *   le foto comparse dopo lo scroll.
 *
 * COME USARLO
 *   1. Apri https://photos.google.com nel browser (Chrome consigliato).
 *   2. Premi F12 (o Cmd+Opt+I su Mac) e vai sulla tab "Console".
 *   3. Incolla TUTTO questo file e premi Invio.
 *   4. Lascia lavorare lo script: scorrerà la pagina da solo fino in fondo.
 *      In console vedrai il conteggio progressivo delle foto selezionate.
 *
 * NOTE
 *   - Con migliaia di foto può richiedere diversi minuti: non chiudere la scheda.
 *   - Lo scroll usa il contenitore interno <c-wiz> (Google non scrolla window).
 *   - Il selettore è "stabile": cerca il tag <c-wiz>, non la classe randomica.
 * ============================================================================= */

(async () => {
  const delay = ms => new Promise(res => setTimeout(res, ms));

  // Selettore stabile: primo C-WIZ scrollabile
  const scrollEl = [...document.querySelectorAll('c-wiz')]
    .find(el => {
      const s = window.getComputedStyle(el);
      return (s.overflowY === 'auto' || s.overflowY === 'scroll')
             && el.scrollHeight > el.clientHeight;
    });

  if (!scrollEl) {
    console.error('❌ Contenitore C-WIZ scrollabile non trovato!');
    return;
  }

  const mainDiv = document.querySelector('[role="main"]');
  if (!mainDiv) {
    console.error('❌ Div [role="main"] non trovato!');
    return;
  }

  console.log('📦 Scroll container trovato | scrollHeight:', scrollEl.scrollHeight);
  console.log('🚀 Avvio selezione foto...');

  let totalSelected = 0;
  let lastScrollHeight = 0;
  let noChangeCount = 0;

  const selectVisible = async () => {
    const checkboxes = mainDiv.querySelectorAll('div[role="checkbox"]');
    let newSelected = 0;

    for (const cb of checkboxes) {
      const label = cb.getAttribute('aria-label') || '';
      const alreadyChecked = cb.getAttribute('aria-checked') === 'true';

      if (!alreadyChecked && !label.startsWith('Salva tutte le foto')) {
        cb.click();
        newSelected++;
        await delay(80); // piccola pausa tra un click e l'altro
      }
    }

    totalSelected += newSelected;
    if (newSelected > 0) {
      console.log(`✅ +${newSelected} selezionate | Totale: ${totalSelected}`);
    }
    return newSelected;
  };

  while (true) {
    // Seleziona le foto attualmente visibili
    await selectVisible();
    await delay(600);

    // Secondo passaggio: riseleziona eventuali missed
    await selectVisible();
    await delay(400);

    // Scrolla verso il basso
    scrollEl.scrollBy({ top: 1200, behavior: 'smooth' });

    // Attesa più lunga per il caricamento lazy delle immagini
    await delay(2000);

    // Seleziona subito dopo lo scroll
    await selectVisible();
    await delay(800);

    // Secondo passaggio post-scroll per sicurezza
    await selectVisible();

    const currentScrollHeight = scrollEl.scrollHeight;

    if (currentScrollHeight === lastScrollHeight) {
      noChangeCount++;
      console.log(`⏳ Nessun nuovo contenuto... (${noChangeCount}/6)`);

      if (noChangeCount >= 6) {
        const atBottom = scrollEl.scrollTop + scrollEl.clientHeight >= scrollEl.scrollHeight - 100;
        if (atBottom) {
          // Passaggio finale triplo per sicurezza
          await delay(2000);
          await selectVisible();
          await delay(1000);
          await selectVisible();
          await delay(1000);
          await selectVisible();
          console.log(`🎉 Completato! Foto totali selezionate: ${totalSelected}`);
          break;
        }
      }
      await delay(2500);
    } else {
      noChangeCount = 0;
      lastScrollHeight = currentScrollHeight;
    }

    await delay(500);
  }
})();
