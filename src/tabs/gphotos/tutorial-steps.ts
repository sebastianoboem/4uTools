export interface TutorialStep {
  target: string
  html: string
}

export const TUTORIAL_STEPS: TutorialStep[] = [
  {
    target: '#access-section',
    html: 'Premi qua per eseguire l&rsquo;accesso'
  },
  {
    target: '#session-section',
    html:
      'In questa sezione potrai decidere di salvare o meno le sessioni ed eliminarle in un secondo momento. Se la voce selezionata è «Temporanea», il programma non salverà nulla della sessione.'
  },
  {
    target: '#btn-start',
    html: 'Avvia la selezione'
  },
  {
    target: '#btn-delete',
    html: 'Seleziona e poi elimina. Se «Scarica» è attivo, prima scarica e poi elimina.'
  },
  {
    target: '.download-option',
    html: 'Con Avvia: seleziona e poi scarica. Con Elimina: seleziona, scarica e poi elimina.'
  },
  {
    target: '#zoom-section',
    html:
      'Qua potrai gestire lo zoom: ti permetterà di vedere più foto alla volta e anche velocizzare il processo. Regola i parametri sottostanti di conseguenza.'
  },
  {
    target: '#params-section',
    html:
      'Qua potrai regolare i parametri fondamentali per gestire l&rsquo;efficacia del software.<br><br><strong>ATTENZIONE:</strong> se non sei sicuro di cosa faccia cosa, lascia pure tutto come predefinito.'
  }
]

export const TUTORIAL_INTRO_HTML =
  'Benvenuto in Google Foto Manager! Segui questo breve tutorial per scoprire come usare il pannello, oppure salta e inizia subito.'
